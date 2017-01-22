//! Simply `println!()`s ticks to the console; useful for debugging.

use trading::tick::Tick;

use super::TickSink;

pub struct ConsoleSink {}

impl TickSink for ConsoleSink {
    fn tick(&mut self, t: Tick) {
        println!("{:?}", t);
    }
}
