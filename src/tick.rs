use serde_json;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tick {
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64
}

impl Tick {
    // returns a dummy placeholder tick
    pub fn null() -> Tick {
        Tick {bid: 0f64, ask: 0f64, timestamp: 0i64}
    }

    // convertes a JSON-encoded String into a Tick
    pub fn from_string(s: String) -> Result<Tick, serde_json::Error> {
        serde_json::from_str(s.as_str())
    }

    // returns the difference between the bid and the ask
    pub fn spread(&self) -> f64 {
        self.bid - self.ask
    }

    // returns the average of the bid and ask price
    pub fn mid(&self) -> f64 {
        (self.bid + self.ask) / 2f64
    }
}
