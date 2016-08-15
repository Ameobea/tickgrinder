#[derive(Debug, Clone, Copy)]
pub struct Tick {
    pub price: f64,
    pub timestamp: i64
}

impl Tick {
    // returns a dummy placeholder tick
    pub fn null() -> Tick {
        Tick {price: 0f64, timestamp: 0i64}
    }
}
