//! A TickGenerator that generates random ticks.

use futures::stream::Receiver;
use algobot_util::trading::tick::Tick;

use data::*;

pub struct RandomReader{}

impl TickGenerator for RandomReader {
    fn get(&mut self, symbol: String) -> Result<Receiver<Tick, ()>, String> {
        unimplemented!();
    }

    fn get_name(&self) -> &'static str {
        "Random"
    }
}
