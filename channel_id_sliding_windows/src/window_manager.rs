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

/// Contains the ticks with the max/min price, and the tick with the oldest timestamp that fits
/// in the window along with its index in the master VecDeque.
type Window = ((Option<Tick>, Option<Tick>), Option<(i64, Tick)>);

pub struct WindowManager {
    pub data: VecDeque<Tick>,
    pub windows: HashMap<i64, Window>,
    pub max_period: Option<i64>
}

/// Determines whether a tick is out of range of a window or not
fn out_of_range(period: i64, oldest_tick: Option<(i64, Tick)>, newest_tick: Tick) -> bool {
    if oldest_tick.is_none() {
        return false
    }
    newest_tick.timestamp - oldest_tick.unwrap().1.timestamp > period
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
        self.windows.insert(period, ((None, None), None));
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

    pub fn push(&mut self, t: Tick) {
        // data is pushed on the back and popped off the front
        self.data.push_back(t);
        self.trim_overflow(t);

        // TODO: return info about new mins/maxes for periods
    }

    /// Removes ticks from the front of the queue until all timestamps are within
    /// the maximum period of the WindowManager.  For each tick that is removed,
    /// check to see if it was the maximum/minimum of any windows.  If it was,
    /// update that window.
    fn trim_overflow(&mut self, newest_tick: Tick) {
        // iterate over all windows and check if the tick is out of their range
        //
        // NOTE: Currently windows are iterated over in an arbitrary order making it impossible to
        // do optimizations like use the max of smaller windows to avoid having to iterate over
        // those parts of larger windows.
        for (&period, &((mut max_price_t, mut min_price_t), mut oldest_t_opt)) in self.windows.iter() {
            // if our current oldest tick is no longer in range of the window
            if out_of_range(period, oldest_t_opt, newest_tick) {
                oldest_t_opt = None;
                // index of previous oldest tick
                let mut i = oldest_t_opt.unwrap().0 + 1;
                // loop through ticks until we find one that's inside the window
                while let Some(newer_tick) = self.data.get(i as usize) {
                    i += 1;
                    if !out_of_range(period, Some((i, *newer_tick)), newest_tick) {
                        oldest_t_opt = Some((i, *newer_tick));
                        break;
                    } else {
                        // update the max/min prices if they were in an out-of-range tick
                        if max_price_t.is_some() && max_price_t.unwrap() == *newer_tick {
                            max_price_t = None
                        }
                        if min_price_t.is_some() && min_price_t.unwrap() == *newer_tick {
                            min_price_t = None
                        }
                    }
                }
                // at this point the oldest tick for the window is either None or accurate
                // (not accounting for the possibility of the newest tick)
            }

            if min_price_t == None || max_price_t == None {
                // must be at least one tick so we can .unwrap()
                let (min_price_t, max_price_t) = self.get_extreme_ticks(period).unwrap();
            } else {
                // check if newest_tick is a new max/min tick for the period
                if newest_tick.mid() < min_price_t.unwrap().mid() {
                    min_price_t = Some(newest_tick);
                }
                if newest_tick.mid() > max_price_t.unwrap().mid() {
                    max_price_t = Some(newest_tick);
                }
            }
        }

        // drop ticks that are outside of the range of the largest window
        while out_of_range(self.max_period.unwrap(), Some((0, *self.data.front().unwrap())), newest_tick) {
            self.data.pop_front();
        }
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
    // TODO
}
