//! Simply `println!()`s ticks to the console; useful for debugging.

use std::fmt::Debug;
use std::collections::HashMap;

use trading::tick::{Tick, GenTick};
use transport::tickstream::{TickSink, GenTickSink};

pub struct ConsoleSink {}

impl TickSink for ConsoleSink {
    fn tick(&mut self, t: Tick) {
        println!("{:?}", t);
    }
}

impl<T> GenTickSink<T> for ConsoleSink where T:Debug, T:Sized {
    fn new(_: HashMap<String, String>) -> Result<ConsoleSink, String> {
        Ok(ConsoleSink {})
    }

    fn tick(&mut self, t: GenTick<T>) {
        println!("{:?}", t);
    }
}
