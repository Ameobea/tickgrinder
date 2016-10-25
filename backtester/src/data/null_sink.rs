//! Pipes data into the abyss.

use algobot_util::trading::tick::Tick;

use data::TickSink;

pub struct NullSink {}

impl TickSink for NullSink {
    fn tick(&mut self, t: Tick) {}
}
