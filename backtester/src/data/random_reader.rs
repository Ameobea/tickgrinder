//! A TickGenerator that generates random ticks.

use futures::Future;
use futures::stream::{channel, Receiver};
use algobot_util::trading::tick::Tick;

use std::thread;

use rand;
use rand::distributions::{IndependentSample, Range};

use data::*;
use backtest::BacktestMap;

pub struct RandomReader{
    pub symbol: String
}

impl TickGenerator for RandomReader {
    fn get(
        &mut self, mut map: Box<BacktestMap + Send>, handle: CommandStream
    )-> Result<Receiver<Tick, ()>, String> {
        let (mut tx, rx) = channel::<Tick, ()>();
        let mut timestamp = 0;

        thread::spawn(move || {
            loop {
                timestamp += 1;

                let mut rng = rand::thread_rng();
                let price_range = Range::new(1f64, 99f64);
                let spread_range = Range::new(0f64, 0.5f64);

                let price = price_range.ind_sample(&mut rng);
                let spread = spread_range.ind_sample(&mut rng);

                let t = Tick {
                    timestamp: timestamp,
                    bid: price,
                    ask: price-spread,
                };

                // apply the map
                let mod_t = map.map(t);
                if mod_t.is_some() {
                    tx = tx.send(Ok(mod_t.unwrap())).wait().ok().unwrap();
                }
            }
        });

        Ok(rx)
    }
}

impl RandomReader {
    pub fn new(symbol: String) -> RandomReader {
        RandomReader {
            symbol: symbol
        }
    }
}
