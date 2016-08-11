use std::slice::Iter;

#[derive(Debug)]
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

pub struct TickField<'tf> {
    symbol: &'tf str,
    ticks: Vec<Tick>
}

impl<'tf> TickField<'tf> {
    pub fn new(symbol: &'tf str) -> TickField<'tf> {
        TickField {
            symbol: symbol,
            ticks: Vec::new()
        }
    }

    pub fn push(&mut self, t: Tick) {
        self.ticks.push(t);
    }

    pub fn iter(&mut self) -> Iter<Tick> {
        return self.ticks.iter();
    }
}
