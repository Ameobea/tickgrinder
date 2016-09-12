//! Manages the sliding windows.  Each time a tick is pushed in, the max/min price of each
//! of the monitored time perdiods is updated.  When ticks become older than the period of
//! the greatest window-newest tick, they are dropped off the structure.
//!
//! NOTE: This algorithm could be more efficiently impelented by a joint linked-list/binary-heap.
//! Traversing a Vector is currently used instead due to the complexity of implementing the
//! alternative.  However, if the latency and inefficniecy caused ever becomes a problem,
//! there are options available.

use std::collections::{HashMap, VecDeque};
use test;

use algobot_util::tick::Tick;

/// Contains the computed information about the ticks in each time monitored time frame.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Window {
    pub min_price_t: Tick,
    pub max_price_t: Tick,
    pub oldest_t:    Tick
}

impl Window {
    pub fn new_empty() -> Window {
        Window {
            min_price_t: Tick::null(),
            max_price_t: Tick::null(),
            oldest_t:    Tick::null()
        }
    }
}

pub struct WindowManager {
    pub data: VecDeque<Tick>,
    pub windows: HashMap<i64, Window>,
    pub max_period: i64
}

/// Determines whether a tick is out of range of a window or not
fn out_of_range(period: i64, old_tick: Tick, newest_tick: Tick) -> bool {
    newest_tick.timestamp - old_tick.timestamp >= period
}

/// Checks if a tick is a max/min tick of a Window
fn is_max_min(t: Tick, window: Window) -> bool {
    if window.min_price_t.timestamp == 0 {
        // doesn't matter since we already have a recalc queued
        return false
    }
    t == window.min_price_t || t == window.max_price_t
}

impl WindowManager {
    pub fn new() -> WindowManager {
        WindowManager {
            data: VecDeque::new(),
            windows: HashMap::new(),
            max_period: 0
        }
    }

    pub fn add_period(&mut self, period: i64) {
        self.windows.insert(period, Window::new_empty());
        // update max period
        if period > self.max_period {
            self.max_period = period;
        }
    }

    pub fn remove_period(&mut self, period: i64) {
        let mut o = 0;
        for (k, _) in self.windows.iter() {
            if *k == period {
                o = *k;
                break;
            }
        }
        if o != 0 {
            self.windows.remove(&o);
            // assume that max_period is already set since we're removing an element
            if self.max_period == period {
                // calculate new max period
                let mut max = 0;
                for (k, _) in self.windows.iter() {
                    if *k > max{
                        max = *k;
                    }
                }
                self.max_period = max;
            }
        }
    }

    // Processes a new tick and returns a Vec containing periods whose mins/maxes have changed.
    pub fn push(&mut self, t: Tick) -> HashMap<i64, Window> {
        assert!(self.windows.len() > 0);
        if self.data.len() > 0 {
            assert!(t.timestamp > self.data.back().unwrap().timestamp);
        }
        // data is pushed on the back and popped off the front
        self.data.push_back(t);

        let mut ret = HashMap::new();
        for (period, window) in self.trim_overflow(t) {
            ret.insert(period, window);
        }

        ret
    }

    /// Removes ticks from the front of the queue until all timestamps are within
    /// the maximum period of the WindowManager.  For each tick that is removed,
    /// check to see if it was the maximum/minimum of any windows.  If it was,
    /// update that window.
    fn trim_overflow(&mut self, newest_tick: Tick) -> Vec<(i64, Window)> {
        let mut altered_periods = Vec::new();

        // iterate over all windows and check if the tick is out of their range
        //
        // NOTE: Currently windows are iterated over in an arbitrary order making it impossible to
        // do optimizations like use the max of smaller windows to avoid having to iterate over
        // those parts of larger windows.
        for (&period, &window) in self.windows.iter() {
            let mut window = window;
            let mut altered = false;
            // if it's the first tick
            if window.oldest_t.timestamp == 0 {
                window.oldest_t = newest_tick;
                window.min_price_t = newest_tick;
                window.max_price_t = newest_tick;
                altered = true;
            } else {
                // if our current oldest tick is no longer in range of the window
                if out_of_range(period, window.oldest_t, newest_tick) {
                    // have to re-scan the entire array starting at the oldest
                    let mut i = 0;

                    // loop through ticks until we find one that's inside the window
                    while let Some(newer_tick) = self.data.get(i as usize) {
                        i += 1;
                        if !out_of_range(period, *newer_tick, newest_tick) {
                            // found the first in-range tick
                            window.oldest_t = *newer_tick;
                            if is_max_min(*newer_tick, window) {
                                window.min_price_t.timestamp = 0;
                                window.max_price_t.timestamp = 0;
                            }
                            break;
                        }

                        // If the tick that went out of range max/min needs updating
                        if is_max_min(*newer_tick, window) {
                            window.min_price_t.timestamp = 0;
                            window.max_price_t.timestamp = 0;
                        }
                    }
                    // at this point the oldest tick for the window is either 0 or accurate
                    // (not accounting for the possibility of the newest tick)
                }
            }

            if window.min_price_t.timestamp == 0 || window.max_price_t.timestamp == 0 {
                let extremes = self.get_extreme_ticks(period);
                window.min_price_t = extremes.0;
                window.max_price_t = extremes.1;
                altered = true;
            } else {
                // check if newest_tick is a new max/min tick for the period
                if newest_tick.mid() < window.min_price_t.mid() {
                    window.min_price_t = newest_tick;
                    altered = true;
                }
                if newest_tick.mid() > window.max_price_t.mid() {
                    window.max_price_t = newest_tick;
                    altered = true;
                }
            }

            if altered {
                // Add the mutated window to the alteration list
                altered_periods.push((period, window));
            }
        }

        // make the alterations queued in the loop
        for &(period, window) in altered_periods.iter() {
            self.windows.insert(period, window);
        }

        // drop ticks that are outside of the range of the largest window
        while out_of_range(self.max_period, *self.data.front().unwrap(), newest_tick) {
            self.data.pop_front();
        }

        altered_periods
    }

    /// Gets the max and min tick in a window by iterating over every tick it contains.
    ///
    /// Returns (min_tick, max_tick)
    fn get_extreme_ticks(&self, period: i64) -> (Tick, Tick) {
        let newest_timestamp = self.data.back().unwrap().timestamp;
        let mut max_tick = Tick::null();
        let mut min_tick = Tick::null();
        let mut max_price = 0f64;
        let mut min_price = 0f64;

        // loop over all ticks, newest to oldest
        for t in self.data.iter().rev() {
            if newest_timestamp - t.timestamp >= period {
                break;
            }

            let price = t.mid();
            if price > max_price {
                max_tick = *t;
                max_price = price;
            }

            if price < min_price || min_price == 0f64 {
                min_tick = *t;
                min_price = price;
            }
        }

        (min_tick, max_tick)
    }
}

#[test]
fn period_addition_removal() {
    let mut wm = WindowManager::new();
    wm.add_period(2);
    wm.add_period(4);
    assert_eq!(wm.windows.len(), 2);
    wm.remove_period(2);
    wm.remove_period(4);
}

#[test]
fn min_max_accuracy() {
    let mut wm = WindowManager::new();
    wm.add_period(2);
    wm.add_period(4);

    let t1 = Tick{timestamp: 1i64, bid: 0.5f64, ask: 0.5f64};
    let wl = wm.push(t1);
    assert_eq!(*wl.get(&2).unwrap(), Window{min_price_t: t1, max_price_t: t1, oldest_t: t1});
    assert_eq!(*wl.get(&4).unwrap(), Window{min_price_t: t1, max_price_t: t1, oldest_t: t1});

    let t2 = Tick{timestamp: 2i64, bid: 2f64, ask: 2f64};
    let w2 = wm.push(t2);
    assert_eq!(*w2.get(&2).unwrap(), Window{min_price_t: t1, max_price_t: t2, oldest_t: t1});
    assert_eq!(*w2.get(&4).unwrap(), Window{min_price_t: t1, max_price_t: t2, oldest_t: t1});

    let t3 = Tick{timestamp: 3i64, bid: 4f64, ask: 4f64};
    let w3 = wm.push(t3);
    assert_eq!(*w3.get(&2).unwrap(), Window{min_price_t: t2, max_price_t: t3, oldest_t: t2});
    assert_eq!(*w3.get(&4).unwrap(), Window{min_price_t: t1, max_price_t: t3, oldest_t: t1});

    let t4 = Tick{timestamp: 6i64, bid: 5f64, ask: 5f64};
    let w4 = wm.push(t4);
    assert_eq!(*w4.get(&2).unwrap(), Window{min_price_t: t4, max_price_t: t4, oldest_t: t4});
    assert_eq!(*w4.get(&4).unwrap(), Window{min_price_t: t3, max_price_t: t4, oldest_t: t3});
}

#[bench]
fn small_wm_processing_speed(b: &mut test::Bencher) {
    let mut wm = WindowManager::new();
    wm.add_period(500);
    // preload with 500,000 ticks
    for i in 0..500 {
        wm.push(Tick{timestamp: i as i64, bid: 1f64, ask: 1f64});
    }
    let mut i = 500;
    b.iter(|| {
        i = i + 1;
        wm.push(Tick{timestamp: i as i64, bid: 1f64, ask: 1f64});
    });
}


// Commented because it takes a while to run
// #[bench]
// fn large_wm_processing_speed(b: &mut test::Bencher) {
//     let mut wm = WindowManager::new();
//     wm.add_period(5000000);
//     // preload with 500,000 ticks
//     for i in 0..5000000 {
//         wm.push(Tick{timestamp: i as i64, bid: 1f64, ask: 1f64});
//     }
//     let mut i = 5000000;
//     b.iter(|| {
//         i = i + 1;
//         wm.push(Tick{timestamp: i as i64, bid: 1f64, ask: 1f64});
//     });
// }
