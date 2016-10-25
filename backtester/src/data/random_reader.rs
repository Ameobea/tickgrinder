//! A TickGenerator that generates random ticks.

use futures::stream::Receiver;
use algobot_util::trading::tick::Tick;

use data::*;
use backtest::BacktestMap;

pub struct RandomReader{
    pub symbol: String
}

impl TickGenerator for RandomReader {
    const NAME: &'static str = "Random";

    fn get(&mut self, map: &BacktestMap) -> Result<Receiver<Tick, ()>, String> {
        unimplemented!();
    }

    fn get_symbol(&self) -> String { self.symbol.clone() }
}
