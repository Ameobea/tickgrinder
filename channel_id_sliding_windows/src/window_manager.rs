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
#[derive(Debug, Copy, Clone)]
pub struct Window {
    pub min_price_t: Option<Tick>,
    pub max_price_t: Option<Tick>,
    pub oldest_t:    Option<(i64, Tick)>
}

impl Window {
    pub fn new_empty() -> Window {
        Window {
            min_price_t: None,
            max_price_t: None,
            oldest_t:   None
        }
    }
}

pub struct WindowManager {
    pub data: VecDeque<Tick>,
    pub windows: HashMap<i64, Window>,
    pub max_period: Option<i64>
}

/// Determines whether a tick is out of range of a window or not
fn out_of_range(period: i64, oldest_tick: Tick, newest_tick: Tick) -> bool {
    newest_tick.timestamp - oldest_tick.timestamp > period
}

impl WindowManager {
    pub fn new() -> WindowManager {
        WindowManager {
            data: VecDeque::new(),
            windows: HashMap::new(),
            max_period: None
        }
    }

    pub fn add_period(&mut self, period: i64) {
        self.windows.insert(period, Window::new_empty());
        // update max period
        match self.max_period {
            Some(_max_period) => {
                if period > _max_period {
                    self.max_period = Some(period);
                }
            },
            None => self.max_period = Some(period)
        }
    }

    pub fn remove_period(&mut self, period: i64) {
        let mut o: Option<i64> = None;
        for (k, _) in self.windows.iter() {
            if *k == period {
                o = Some(*k);
                break;
            }
        }
        if o.is_some() {
            self.windows.remove(&o.unwrap());
            // assume that max_period is already set since we're removing an element
            if self.max_period.unwrap() == period {
                // calculate new max period
                let mut max: Option<i64> = None;
                for (k, _) in self.windows.iter() {
                    if max.is_none() || *k > max.unwrap() {
                        max = Some(*k);
                    }
                }
                self.max_period = max;
            }
        }
    }

    // Processes a new tick and returns a Vec containing periods whose mins/maxes have changed.
    pub fn push(&mut self, t: Tick) -> Vec<(i64, Window)> {
        // data is pushed on the back and popped off the front
        self.data.push_back(t);
        self.trim_overflow(t)
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
            // println!("Window before: {:?}", window);
            let mut altered = false;
            // if it's the first tick
            if window.oldest_t.is_none() {
                window.oldest_t = Some((0, newest_tick));
            } else {
                // if our current oldest tick is no longer in range of the window
                if out_of_range(period, window.oldest_t.unwrap().1, newest_tick) {
                    // index of previous oldest tick
                    let mut i = window.oldest_t.unwrap().0 + 1;

                    // loop through ticks until we find one that's inside the window
                    while let Some(newer_tick) = self.data.get(i as usize) {
                        i += 1;
                        if !out_of_range(period, *newer_tick, newest_tick) {
                            window.oldest_t = Some((i, *newer_tick));
                            break;
                        } else {
                            // update the max/min prices if they were in an out-of-range tick
                            if window.max_price_t.is_some() && window.max_price_t.unwrap() == *newer_tick {
                                window.max_price_t = None;
                                altered = true;
                            }
                            if window.min_price_t.is_some() && window.min_price_t.unwrap() == *newer_tick {
                                window.min_price_t = None;
                                altered = true;
                            }
                        }
                    }
                    // at this point the oldest tick for the window is either None or accurate
                    // (not accounting for the possibility of the newest tick)
                }
            }

            if window.min_price_t == None || window.max_price_t == None {
                // there is at least one tick so we can .unwrap()
                let extremes = self.get_extreme_ticks(period).unwrap();
                window.min_price_t = Some(extremes.0);
                window.max_price_t = Some(extremes.1);
                altered = true;
            } else {
                // check if newest_tick is a new max/min tick for the period
                if newest_tick.mid() < window.min_price_t.unwrap().mid() {
                    window.min_price_t = Some(newest_tick);
                    altered = true;
                }
                if newest_tick.mid() > window.max_price_t.unwrap().mid() {
                    window.max_price_t = Some(newest_tick);
                    altered = true;
                }
            }

            if altered {
                // Add the mutated window to the alteration list
                altered_periods.push((period, window));
                // println!("Window after: {:?}", window);
            }
        }

        // make the alterations queued in the loop
        for &(period, window) in altered_periods.iter() {
            self.windows.insert(period, window);
        }

        // drop ticks that are outside of the range of the largest window
        while out_of_range(self.max_period.unwrap(), *self.data.front().unwrap(), newest_tick) {
            self.data.pop_front();
        }

        altered_periods
    }

    /// Gets the max and min tick in a window by iterating over every tick it contains.
    ///
    /// Returns Some(min_tick, max_tick)
    fn get_extreme_ticks(&self, period: i64) -> Option<(Tick, Tick)> {
        let newest_timestamp = self.data.back().unwrap().timestamp;
        let mut max_tick = Tick::null();
        let mut min_tick = Tick::null();
        let mut max_price = 0f64;
        let mut min_price = 0f64;

        for t in self.data.iter() {
            if newest_timestamp - t.timestamp > period {
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

        if min_price == 0f64 {
            return None
        }

        Some((min_tick, max_tick))
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
    wm.add_period(5);
    wm.add_period(10);
    println!("result of push: {:?}", wm.push(Tick{timestamp: 1i64, bid: 1f64, ask: 1f64}));
    println!("result of push: {:?}", wm.push(Tick{timestamp: 3i64, bid: 2f64, ask: 2f64}));
    // println!("result of push: {:?}", wm.push(Tick{timestamp: 1i64, bid: 1f64, ask: 1f64}));
    // println!("result of push: {:?}", wm.push(Tick{timestamp: 1i64, bid: 1f64, ask: 1f64}));
    panic!("{:?}", "s");
}

#[bench]
fn large_wm_processing_speed(b: &mut test::Bencher) {
    let mut wm = WindowManager::new();
    wm.add_period(5000000);
    // preload with 500,000 ticks
    for i in 0..5000000 {
        wm.push(Tick{timestamp: i as i64, bid: 1f64, ask: 1f64});
    }
    let mut i = 5000000;
    b.iter(|| {
        i = i + 1;
        wm.push(Tick{timestamp: i as i64, bid: 1f64, ask: 1f64});
    });
}
