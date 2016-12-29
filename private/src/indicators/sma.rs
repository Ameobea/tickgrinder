use std::collections::{VecDeque, HashMap};
use std::error::Error;

#[allow(unused_imports)]
use test;
use postgres::rows::Row;
use serde_json;

use algobot_util::trading::indicators::*;
use algobot_util::trading::tick::*;
use algobot_util::transport::postgres::*;
#[allow(unused_imports)]
use algobot_util::trading::datafield::DataField;

/// Alteration of a simple moving average using ticks as input where the prices in a time frame
/// are weighted by the time the price stayed at that level before changing.
pub struct Sma {
    pub period: usize,
    pub ticks: VecDeque<Tick>,
    // indicates if an out-of-range tick exists in the front element
    ref_tick: Tick,
}

impl Sma {
    pub fn new(period: usize) -> Sma {
        Sma {
            period: period,
            ticks: VecDeque::new(),
            ref_tick: Tick::null(),
        }
    }

    /// Trims out of range ticks from the front of the queue.
    /// Returns the last out-of-range tick removed.
    fn trim(&mut self) -> Tick {
        let mut t: Tick = Tick::null();
        while self.is_overflown() {
            t = self.ticks.pop_front().unwrap();
        }

        t
    }

    /// Returns the average price for the SMA's period.
    fn average(&self) -> usize {
        let mut p_sum = 0; // sum of prices
        let mut t_sum = 0; // sum of time
        let mut iter = self.ticks.iter();
        iter.next(); // skip first value since there's no time difference to compute
        let mut last_tick = self.ticks.front().unwrap();
        // loop over ticks, oldest to newest
        for t in iter {
            let t_diff = t.timestamp - last_tick.timestamp;
            p_sum += last_tick.mid() * t_diff;
            t_sum += t_diff;
            last_tick = t;
        }

        // if there is a previous value to take into account
        if self.ref_tick.bid != 0 {
            let old_time = self.period - t_sum;
            p_sum += old_time * self.ref_tick.mid();
            t_sum = self.period;
        }

        p_sum / t_sum
    }

    fn is_overflown(&self) -> bool {
        // time between newest tick and reference tick
        let diff = self.ticks.back().unwrap().timestamp - self.ticks.front().unwrap().timestamp;
        diff >= self.period
    }

    /// Add a new tick to be averaged.
    pub fn push(&mut self, t: Tick) -> usize {
        // open new section so we're not double-borrowing self.ticks
        {
            let last_tick: Option<&Tick> = self.ticks.back();
            if last_tick.is_some() {
                assert!(t.timestamp > last_tick.unwrap().timestamp, "Out-of-order ticks sent to SMA!
                    timestamps: {:?}, {:?}", last_tick.unwrap().timestamp, t.timestamp);
            }
        }
        self.ticks.push_back(t);

        if self.is_overflown() {
            self.ref_tick = self.trim();
        }

        if self.ticks.len() == 1 {
            return self.ticks.front().unwrap().mid()
        }

        self.average()
    }

    /// Same as push but returns a tick representing the average bid and ask instead a usize.
    pub fn push_tick(&mut self, t: Tick) -> Tick {
        let _ = self.push(t);
        self.average_tick()
    }

    pub fn average_tick(&self) -> Tick {
        if self.ticks.len() == 1 {
            return *self.ticks.front().unwrap()
        }

        let mut bid_sum = 0;
        let mut ask_sum = 0;
        let mut t_sum = 0; // sum of time
        let mut iter = self.ticks.iter();
        iter.next(); // skip first value since there's no time difference to compute
        let mut last_tick = self.ticks.front().unwrap();
        // loop over ticks, oldest to newest
        for t in iter {
            let t_diff = (t.timestamp - last_tick.timestamp) as usize;
            bid_sum += last_tick.bid * t_diff;
            ask_sum += last_tick.ask * t_diff;
            t_sum += t_diff;
            last_tick = t;
        }

        // if there is a previous value to take into account
        if self.ref_tick.bid != 0 {
            let old_time = self.period - t_sum;
            bid_sum += old_time * self.ref_tick.bid;
            ask_sum += old_time * self.ref_tick.ask;
            t_sum = self.period;
        }

        Tick {
            bid: bid_sum / t_sum,
            ask: ask_sum / t_sum,
            timestamp: (*self.ticks.back().unwrap()).timestamp,
        }
    }
}

impl HistQuery for Sma {
    /// Queries the database for ticks in a range and returns the average bid and ask in that range.
    fn get(start_time: usize, end_time: usize, period: usize, args: HashMap<String, String>) -> Result<String, String> {
        let connection_opt = get_client();
        if connection_opt.is_err() {
            return Err(String::from("Unable to connect to PostgreSQL!"))
        }
        let conn = connection_opt.unwrap();

        let table_name = try!( args.get("table_name").ok_or(no_arg_error("table_name")) );

        let query = format!(
            "SELECT (tick_time, bid, ask) FROM {} WHERE tick_time > {} AND tick_time < {};",
            table_name,
            start_time,
            end_time
        );
        let rows = try!(
            conn.query(&query, &[]).map_err( |err| format!("{}", err) )
        );

        let mut sma = Sma::new(period);
        let mut last_time = 0;
        let mut res = Vec::new();

        for row in rows.iter() {
            let t: Tick = tick_from_row(&row);
            let res_t = sma.push_tick(t);

            if last_time == 0 || (t.timestamp - last_time) > period {
                res.push(res_t);
                last_time = t.timestamp;
            }
        }

        serde_json::to_string(&res).map_err(|err| String::from(err.description()) )
    }
}

/// Takes a row and returns a tick from the values of its columns.
/// Panics if the row doesn't contain properly formatted `tick_time`, `bid`, and `ask` columns.
fn tick_from_row(row: &Row) -> Tick {
    let timestamp = row.get::<_, i64>("tick_time") as usize;
    let bid = row.get::<_, i64>("bid") as usize;
    let ask = row.get::<_, i64>("ask") as usize;

    Tick {
        timestamp: timestamp,
        bid: bid,
        ask: ask,
    }
}

fn no_arg_error(name: &str) -> String {
    format!("No argument \"{}\" provided in the arguments HashMap.", name)
}

#[test]
fn hist_sma_accuracy() {
    // TODO: Write this test >.>
}

#[test]
fn sma_accuracy() {
    let mut sma = Sma::new(15);
    let mut t = Tick {bid: 101, ask: 107, timestamp: 1};
    let mut avg = sma.push(t);
    assert_eq!(avg, t.mid());

    t = Tick {bid: 103, ask: 108, timestamp: 5};
    avg = sma.push(t);
    let man_avg = (101 + 107) / 2;
    assert_eq!(avg, man_avg);

    t = Tick {bid: 105, ask: 109, timestamp: 13};
    avg = sma.push(t);
    let man_avg = ((((101 + 107) / 2) * 4) +
                  (((103 + 108) / 2) * 8)) / 12;
    assert_eq!(avg, man_avg);

    t = Tick {bid: 104, ask: 1088, timestamp: 18};
    avg = sma.push(t);
    let man_avg = ((((103 + 108) / 2) * 8) +
                  (((105 + 109) / 2) * 5) +
                  (((101 + 107) / 2) * 2)) / 15;
    assert_eq!(avg, man_avg);
}

#[test]
fn tick_sma_accuracy() {
    let mut sma = Sma::new(15);
    let mut t = Tick {bid: 101, ask: 107, timestamp: 1};
    let mut avg_t = sma.push_tick(t);
    assert_eq!(avg_t.mid(), t.mid());

    t = Tick {bid: 103, ask: 108, timestamp: 5};
    avg_t = sma.push_tick(t);
    let man_avg = (101 + 107) / 2;
    assert_eq!(avg_t.mid(), man_avg);

    t = Tick {bid: 105, ask: 109, timestamp: 13};
    avg_t = sma.push_tick(t);
    let man_avg = ((((101 + 107) / 2) * 4) +
                  (((103 + 108) / 2) * 8)) / 12;
    assert_eq!(avg_t.mid(), man_avg);

    t = Tick {bid: 104, ask: 1088, timestamp: 18};
    avg_t = sma.push_tick(t);
    let man_avg = ((((103 + 108) / 2) * 8) +
                  (((105 + 109) / 2) * 5) +
                  (((101 + 107) / 2) * 2)) / 15;
    assert_eq!(avg_t.mid(), man_avg);
}

// insert a tick into a DataField
#[bench]
fn tick_insertion(b: &mut test::Bencher) {
    let t = Tick {bid: 1123128412, ask: 1123128402, timestamp: 1471291001837};
    let mut df: DataField<Tick> = DataField::new();

    b.iter(|| {
        let mut df = &mut df;
        df.push(t);
    });
}

#[bench]
fn sma_calculation(b: &mut test::Bencher) {
    let mut sma = Sma::new(15);
    let mut timestamp = 1;

    b.iter(|| {
        sma.push(Tick{bid: 1239123, ask: 112312, timestamp: timestamp});
        timestamp += 1;
    });
}
