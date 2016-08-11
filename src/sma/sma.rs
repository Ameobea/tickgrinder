use std::collections::VecDeque;
use std::slice::Iter;

//Calculate weighted average of all ticks within period seconds
//pop ticks off the front after they leave the period

#[derive(Debug)]
struct Tick {
    price: f64,
    timestamp: i64
}

impl Tick {
    // returns a dummy placeholder tick
    fn null() -> Tick {
        Tick {price: 0f64, timestamp: 0i64}
    }
}

struct TickField<'tf> {
    symbol: &'tf str,
    ticks: Vec<Tick>
}

impl<'tf> TickField<'tf> {
    fn new(symbol: &'tf str) -> TickField<'tf> {
        TickField {
            symbol: symbol,
            ticks: Vec::new()
        }
    }

    fn push(&mut self, t: Tick) {
        self.ticks.push(t);
    }

    fn iter(&mut self) -> Iter<Tick> {
        return self.ticks.iter();
    }
}

struct SimpleMovingAverage<'tf> {
    period: i64,
    ticks: VecDeque<&'tf Tick>,
    // indicates if an out-of-range tick exists in the front element
    overflow: bool,
    // stores the price of the last tick before this series
    ref_tick: Tick
}

impl<'tf> SimpleMovingAverage<'tf> {
    fn new(period: i64) -> SimpleMovingAverage<'tf> {
        SimpleMovingAverage {
            period: period,
            ticks: VecDeque::new(),
            overflow: false,
            ref_tick: Tick::null()
        }
    }

    // trims out of range ticks from the front of the queue
    // returns the last out-of-range tick removed
    fn trim(&mut self) -> Tick {
        let mut t: &Tick = &Tick::null();
        while self.is_overflown() {
            t = self.ticks.pop_front().unwrap()
        }
        return Tick {price: t.price, timestamp: t.timestamp};
    }

    fn average(&self) -> Option<f64> {
        if self.ref_tick.price == 0f64 {return None}
        let mut p_sum: f64 = 0f64; // sum of prices
        let mut t_sum: f64 = 0f64; // sum of time
        let last_timestamp: i64 = self.ticks.back().unwrap().timestamp;
        for t in self.ticks.iter().next() {
            assert!(t.timestamp < last_timestamp, "Out-of-order ticks sent to SMA!
                timestamps: {:?}, {:?}", last_timestamp, t.timestamp);
            println!("{:?}", t);
            let t_diff: f64 = last_timestamp as f64 - t.timestamp as f64;
            p_sum += t.price * t_diff;
            t_sum += t_diff;
        }
        let old_time: f64 = self.period as f64 - t_sum;
        p_sum += old_time * self.ref_tick.price as f64;
        return Some(p_sum / self.period as f64);
    }

    fn is_overflown(&self) -> bool {
        let diff: i64 = self.ticks.back().unwrap().timestamp - self.ticks.front().unwrap().timestamp;
        return diff >= self.period;
    }

    fn push(&mut self, t: &'tf Tick) -> Option<f64> {
        self.ticks.push_back(t);
        if !self.overflow{
            if self.is_overflown() {
                self.overflow = true;
            }
        }else{
            self.ref_tick = self.trim();
        }

        if self.ticks.is_empty() {
            return None;
        }else if self.ticks.len() == 1 {
            return Some(self.ticks.front().unwrap().price);
        }else {
            return self.average();
        }
    }
}

fn main() {
    let mut tf = TickField::new("USDCAD");
    tf.push(Tick{price: 23f64, timestamp: 1470533189000i64});
    tf.push(Tick{price: 23f64, timestamp: 1470533191010i64});
    tf.push(Tick{price: 23.23894f64, timestamp: 1470533192410i64});

    for period in [3, 5].iter() {
        println!("Moving average with period {}", period);

        let mut sma = SimpleMovingAverage::new(*period as i64);
        let mut test: Option<f64> = Some(0f64);
        for t in tf.iter() {
            test = sma.push(t);
        }
        println!("{:?}", test);
    }
}
