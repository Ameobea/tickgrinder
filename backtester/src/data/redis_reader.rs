//! A TickGenerator that reads ticks out of a Redis channel.

use std::thread;

use futures::Future;
use futures::stream::{Stream, channel, Receiver};
use algobot_util::trading::tick::Tick;
use algobot_util::transport::redis::sub_channel;

use data::*;
use backtest::BacktestMap;

pub struct RedisReader {
    pub symbol: String,
    pub redis_host: String,
    pub channel: String
}

impl TickGenerator for RedisReader {
    fn get(
        &mut self, mut map: Box<BacktestMap + Send>, handle: CommandStream
    )-> Result<Receiver<Tick, ()>, String> {
        let host = self.redis_host.clone();
        let input_channel = self.channel.clone();

        let (mut tx, rx) = channel::<Tick, ()>();

        thread::spawn(move || {
            let in_rx = sub_channel(host.as_str(), input_channel.as_str());

            for t_string in in_rx.wait() {
                let t = Tick::from_json_string(t_string.unwrap());

                // apply map
                let t_mod = map.map(t);
                if t_mod.is_some() {
                    tx = tx.send(Ok(t_mod.unwrap())).wait().ok().unwrap();
                }
            }
        });

        Ok(rx)
    }
}

impl RedisReader {
    pub fn new(symbol: String, host: String, channel: String) -> RedisReader {
        RedisReader {
            symbol: symbol,
            redis_host: host,
            channel: channel
        }
    }
}
