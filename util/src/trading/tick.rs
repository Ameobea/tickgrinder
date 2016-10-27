//! Structs and functions for creating and managing Ticks.  Ticks represent one
//! price change in a timeseries.

use serde_json;

use std::str::FromStr;
#[allow(unused_imports)]
use test;

use transport::query_server::QueryServer;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Tick {
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymbolTick {
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64,
    pub symbol: String
}

impl Tick {
    /// Returns a dummy placeholder tick
    pub fn null() -> Tick {
        Tick {bid: 0f64, ask: 0f64, timestamp: 0i64}
    }

    /// Converts a JSON-encoded String into a Tick
    pub fn from_json_string(s: String) -> Tick {
        serde_json::from_str(s.as_str()).expect("Unable to parse tick from string")
    }

    /// generates a JSON string containing the data of the tick
    pub fn to_json_string(&self, symbol :String) -> String {
        serde_json::to_string(&SymbolTick::from_tick(*self, symbol))
            .expect("Couldn't convert tick to json string")
    }

    /// Returns the difference between the bid and the ask
    pub fn spread(&self) -> f64 {
        self.bid - self.ask
    }

    /// Returns the average of the bid and ask price
    pub fn mid(&self) -> f64 {
        (self.bid + self.ask) / 2f64
    }

    /// Saves the tick in the database.
    /// The table "ticks_SYMBOL" must exist.
    pub fn store(&self, symbol: &str, qs: &mut QueryServer) {
        let query = format!(
            "INSERT INTO ticks_{} (tick_time, bid, ask) VALUES ({}, {}, {});",
            symbol,
            self.timestamp,
            self.bid,
            self.ask
        );

        // Asynchronously store the tick in the database
        qs.execute(query);
    }

    /// Converts a SymbolTick into a Tick, dropping the symbol
    pub fn from_symboltick(st: SymbolTick) -> Tick {
        Tick {
            timestamp: st.timestamp,
            bid: st.bid,
            ask: st.ask
        }
    }

    /// Converts a String in the format "{timestamp}, {bid}, {ask}" into a Tick
    pub fn from_csv_string(s: &str) -> Tick {
        let spl: Vec<&str> = s.split(", ").collect();
        Tick {
            timestamp: i64::from_str_radix(spl[0], 10).unwrap(),
            bid: f64::from_str(spl[1]).unwrap(),
            ask: f64::from_str(spl[2]).unwrap()
        }
    }

    /// Converts a f64 price into pips
    pub fn price_to_pips(p: f64) -> usize {
        unimplemented!();
    }
}

impl SymbolTick {
    /// creates a SymbolTick given a Tick and a SymbolTick
    pub fn from_tick(tick: Tick, symbol: String) -> SymbolTick {
        SymbolTick {bid: tick.bid, ask: tick.ask, timestamp: tick.timestamp, symbol: symbol}
    }

    /// Converts a JSON-encoded String into a Tick
    pub fn from_json_string(s: String) -> SymbolTick {
        serde_json::from_str(s.as_str()).expect("Unable to parse tick from string")
    }
}

#[bench]
fn from_csv_string(b: &mut test::Bencher) {
    let s = "1476650327123, 1.23134, 1.23156";
    let mut t = Tick::null();
    let _ = b.iter(|| {
        t = Tick::from_csv_string(s)
    });
}
