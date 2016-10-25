//! A TickGenerator that reads ticks out of a Redis channel.

use futures::stream::Receiver;
use algobot_util::trading::tick::Tick;
use algobot_util::transport::redis::*;

use data::*;
use backtest::BacktestMap;

pub struct RedisReader{
    pub symbol: String
}

impl TickGenerator for RedisReader {
    const NAME: &'static str = "Redis";

    fn get(&mut self, map: &BacktestMap) -> Result<Receiver<Tick, ()>, String> {
        unimplemented!();
    }

    fn get_symbol(&self) -> String { self.symbol.clone() }
}
