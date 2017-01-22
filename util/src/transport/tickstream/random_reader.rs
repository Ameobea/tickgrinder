//! A `TickGenerator` that generates random ticks.

use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use rand::{thread_rng, ThreadRng};
use rand::distributions::{IndependentSample, Range};

use futures::sync::mpsc::{unbounded, UnboundedReceiver};

use trading::tick::Tick;

use super::*;

pub struct RandomReader {}

impl TickGenerator for RandomReader {
    fn get(
        &mut self, mut map: Box<TickMap + Send>, cmd_handle: CommandStream
    ) -> Result<UnboundedReceiver<Tick>, String> {
        let (tx, rx) = unbounded::<Tick>();
        let mut timestamp = 0;

        // small atomic communication bus between the handle listener and worker threads
        let internal_message: Arc<Mutex<TickstreamCommand>> = Arc::new(Mutex::new(TickstreamCommand::Stop));
        let _internal_message = internal_message.clone();
        let got_mail = Arc::new(AtomicBool::new(false));
        let mut _got_mail = got_mail.clone();

        let reader_handle = thread::spawn(move || {
            thread::park();

            let mut rng = thread_rng();
            loop {
                if check_mail(&*got_mail, &*_internal_message) {
                    println!("Stop command received; killing reader");
                    break;
                }
                timestamp += 1;

                let tick = get_rand_tick(&mut rng, timestamp);

                // apply the map
                let mod_t = map.map(tick);
                if mod_t.is_some() {
                    tx.send(mod_t.unwrap()).expect("Unable to send tick to sink in random_reader.rs");
                }
            }
        }).thread().clone();

        // spawn the handle listener thread that awaits commands
        spawn_listener_thread(_got_mail, cmd_handle, internal_message, reader_handle);

        Ok(rx)
    }

    fn get_raw(&mut self) -> Result<UnboundedReceiver<Tick>, String> {
        let (tx, rx) = unbounded();
        let mut timestamp = 0;

        thread::spawn(move || {
            let mut rng = thread_rng();
            loop {
                let t = get_rand_tick(&mut rng, timestamp);
                tx.send(t).unwrap();
                timestamp += 1;
            }
        });

        Ok(rx)
    }
}

fn get_rand_tick(mut rng: &mut ThreadRng, timestamp: u64) -> Tick {
    let price_range = Range::new(10, 99);
    let spread_range = Range::new(0, 5);

    let price = price_range.ind_sample(rng);
    let spread = spread_range.ind_sample(rng);

    Tick {
        timestamp: timestamp,
        bid: price,
        ask: price-spread,
    }
}
