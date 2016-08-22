use serde_json;
use postgres::Connection;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tick {
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SymbolTick {
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64,
    pub symbol: String
}

impl Tick {
    // returns a dummy placeholder tick
    pub fn null() -> Tick {
        Tick {bid: 0f64, ask: 0f64, timestamp: 0i64}
    }

    // converts a JSON-encoded String into a Tick
    pub fn from_json_string(s: String) -> Result<Tick, serde_json::Error> {
        serde_json::from_str(s.as_str())
    }

    // generates a JSON string containing the data of the tick
    pub fn to_json_string(&self, symbol :String) -> String {
        serde_json::to_string(&SymbolTick::from_tick(*self, symbol)).unwrap()
    }

    // returns the difference between the bid and the ask
    pub fn spread(&self) -> f64 {
        self.bid - self.ask
    }

    // returns the average of the bid and ask price
    pub fn mid(&self) -> f64 {
        (self.bid + self.ask) / 2f64
    }

    // saves the tick in the database
    // the table "ticks_SYMBOL" must exist.
    pub fn store(&self, symbol: &str, client: &Connection) {
        let query = format!(
            "INSERT INTO ticks_{} (tick_time, bid, ask) VALUES ({}, {}, {});",
            symbol,
            self.timestamp,
            self.bid,
            self.ask
        );

        client.execute(query.as_str(), &[]).expect("Unable to store tick!");
    }
}

impl SymbolTick {
    // creates a SymbolTick given a Tick and a SymbolTick
    pub fn from_tick(tick: Tick, symbol: String) -> SymbolTick {
        SymbolTick {bid: tick.bid, ask: tick.ask, timestamp: tick.timestamp, symbol: symbol}
    }
}
