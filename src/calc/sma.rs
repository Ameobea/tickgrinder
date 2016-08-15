use std::collections::VecDeque;

use tick::Tick;

//Calculate weighted average of all ticks within period seconds
//pop ticks off the front after they leave the period

pub struct SimpleMovingAverage {
    period: f64,
    ticks: VecDeque<Tick>,
    // indicates if an out-of-range tick exists in the front element
    ref_tick: Tick,
    overflown_once: bool
}

impl SimpleMovingAverage {
    pub fn new(period: f64) -> SimpleMovingAverage {
        SimpleMovingAverage {
            period: period,
            ticks: VecDeque::new(),
            ref_tick: Tick::null(),
            overflown_once: false,
        }
    }

    // trims out of range ticks from the front of the queue
    // returns the last out-of-range tick removed
    fn trim(&mut self) -> Tick {
        let mut t: Tick = Tick::null();
        while self.is_overflown() {
            t = self.ticks.pop_front().unwrap();
        }
        t
    }

    fn average(&self) -> f64 {
        let mut p_sum = 0f64; // sum of prices
        let mut t_sum = 0f64; // sum of time
        let newest_timestamp: i64 = self.ticks.back().unwrap().timestamp;
        let mut iter = self.ticks.iter().enumerate();
        iter.next(); // skip first value since there's no time difference to compute
        let mut last_tick;
        for (i, t) in iter {
            last_tick = self.ticks[i-1];
            let t_diff = (t.timestamp - last_tick.timestamp) as f64;
            p_sum += t.mid() * t_diff;
            t_sum += t_diff;
        }

        // if there is a previous value to take into account
        if self.ref_tick.bid != 0f64 {
            let old_time: f64 = self.period - t_sum;
            p_sum += old_time * self.ref_tick.mid();
            t_sum = self.period;
        }

        println!("{:?}", (p_sum, t_sum));
        p_sum / t_sum
    }

    fn is_overflown(&self) -> bool {
        // time between newest tick and reference tick
        let diff: i64 = self.ticks.back().unwrap().timestamp - self.ticks.front().unwrap().timestamp;
        println!("time between newest tick and ref_tick: {:?}", diff);
        diff as f64 >= self.period
    }

    // Add a new tick to be averaged.
    pub fn push(&mut self, t: Tick) -> f64 {
        // open new section so we're not double-borrowing self.ticks
        {
            let last_tick: Option<&Tick> = self.ticks.back();
            if last_tick.is_some() {
                assert!(t.timestamp > last_tick.unwrap().timestamp, "Out-of-order ticks sent to SMA!
                    timestamps: {:?}, {:?}", last_tick.unwrap().timestamp, t.timestamp);
            }
        }
        self.ticks.push_back(t);
        // if we haven't overflown before
        let is_overflown = self.is_overflown();
        if !self.overflown_once && is_overflown {
            self.overflown_once = true;
        }

        if is_overflown {
            self.ref_tick = self.trim();
            println!("ref tick: {:?}", self.ref_tick);
        }

        if self.ticks.len() == 1 {
            return self.ticks.front().unwrap().mid()
        }

        self.average()
    }
}
