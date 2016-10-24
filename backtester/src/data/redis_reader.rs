//! A TickGenerator that reads ticks out of a Redis channel.

use futures::stream::Receiver;
use algobot_util::trading::tick::Tick;
use algobot_util::transport::redis::*;

use data::*;

pub struct RedisReader{}

impl TickGenerator for RedisReader {
    fn get(&mut self, symbol: String) -> Result<Receiver<Tick, ()>, String> {
        unimplemented!();
    }

    fn get_name(&self) -> &'static str {
        "Random"
    }
}
