//! Structs and functions for creating and managing Ticks.  Ticks represent one
//! price change in a timeseries.

use serde_json;

use std::str::FromStr;
#[allow(unused_imports)]
use test;

use transport::query_server::QueryServer;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Tick {
    pub bid: usize,
    pub ask: usize,
    pub timestamp: usize
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymbolTick {
    pub bid: usize,
    pub ask: usize,
    pub timestamp: usize,
    pub symbol: String
}

impl Tick {
    /// Returns a dummy placeholder tick
    pub fn null() -> Tick {
        Tick {bid: 0usize, ask: 0usize, timestamp: 0usize}
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
    pub fn spread(&self) -> usize {
        self.bid - self.ask
    }

    /// Returns the average of the bid and ask price
    pub fn mid(&self) -> usize {
        (self.bid + self.ask) / 2usize
    }

    /// Saves the tick in the database.  The table "ticks_SYMBOL" must exist.
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

    /// Saves the tick in the specified table.  The table must exist.
    pub fn store_table(&self, table: &str, qs: &mut QueryServer) {
        let query = format!(
            "INSERT INTO {} (tick_time, bid, ask) VALUES ({}, {}, {});",
            table,
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
            timestamp: usize::from_str_radix(spl[0], 10).unwrap(),
            bid: usize::from_str(spl[1]).unwrap(),
            ask: usize::from_str(spl[2]).unwrap()
        }
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
    let s = "1476650327123, 123134, 123156";
    let mut t = Tick::null();
    let _ = b.iter(|| {
        t = Tick::from_csv_string(s)
    });
}
