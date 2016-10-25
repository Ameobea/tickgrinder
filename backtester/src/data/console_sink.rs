//! Simply `println!()`s ticks to the console; useful for debugging.

use algobot_util::trading::tick::Tick;

use data::TickSink;

pub struct ConsoleSink {}

impl TickSink for ConsoleSink {
    const NAME: &'static str = "Console";

    fn tick(&mut self, t: Tick) {
        println!("{:?}", t);
    }
}