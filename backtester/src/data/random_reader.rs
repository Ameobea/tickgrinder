//! A TickGenerator that generates random ticks.

use futures::Future;
use futures::stream::{channel, Receiver};
use algobot_util::trading::tick::Tick;

use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

use rand;
use rand::distributions::{IndependentSample, Range};

use data::*;
use backtest::{BacktestMap, BacktestCommand};

pub struct RandomReader{
    pub symbol: String
}

impl TickGenerator for RandomReader {
    fn get(
        &mut self, mut map: Box<BacktestMap + Send>, cmd_handle: CommandStream
    )-> Result<Receiver<Tick, ()>, String> {
        let (mut tx, rx) = channel::<Tick, ()>();
        let mut timestamp = 0;

        // small atomic communication bus between the handle listener and worker threads
        let internal_message: Arc<Mutex<BacktestCommand>> = Arc::new(Mutex::new(BacktestCommand::Stop));
        let _internal_message = internal_message.clone();
        let got_mail = Arc::new(AtomicBool::new(false));
        let mut _got_mail = got_mail.clone();

        let reader_handle = thread::spawn(move || {
            thread::park();
            loop {
                if check_mail(&*got_mail, &*_internal_message) {
                    println!("Stop command received; killing reader");
                    break;
                }
                timestamp += 1;

                let mut rng = rand::thread_rng();
                let price_range = Range::new(10, 99);
                let spread_range = Range::new(0, 5);

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
        }).thread().clone();

        // spawn the handle listener thread that awaits commands
        spawn_listener_thread(_got_mail, cmd_handle, internal_message, reader_handle);

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

// TODO: Tests for command handling
