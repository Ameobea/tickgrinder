//! A `TickGenerator` that reads ticks out of a Redis channel.

use std::thread;

use futures::Stream;
use futures::sync::mpsc::{unbounded, UnboundedReceiver};

use trading::tick::Tick;
use transport::redis::sub_channel;
use super::*;

pub struct RedisReader {
    pub symbol: String,
    pub redis_host: String,
    pub channel: String
}

impl TickGenerator for RedisReader {
    fn get(
        &mut self, mut map: Box<TickMap + Send>, cmd_handle: CommandStream
    ) -> Result<UnboundedReceiver<Tick>, String> {
        let host = self.redis_host.clone();
        let input_channel = self.channel.clone();

        // small atomic communication bus between the handle listener and worker threads
        let internal_message: Arc<Mutex<TickstreamCommand>> = Arc::new(Mutex::new(TickstreamCommand::Stop));
        let _internal_message = internal_message.clone();
        let got_mail = Arc::new(AtomicBool::new(false));
        let mut _got_mail = got_mail.clone();

        let (tx, rx) = unbounded::<Tick>();

        let reader_handle = thread::spawn(move || {
            let in_rx = sub_channel(host.as_str(), input_channel.as_str());

            for t_string in in_rx.wait() {
                if check_mail(&*got_mail, &*_internal_message) {
                    println!("Stop command received; killing reader");
                    break;
                }
                let t = Tick::from_json_string(t_string.unwrap());

                // apply map
                let t_mod = map.map(t);
                if t_mod.is_some() {
                    tx.send(t_mod.unwrap()).unwrap();
                }
            }
        }).thread().clone();

        // spawn the handle listener thread that awaits commands
        spawn_listener_thread(_got_mail, cmd_handle, internal_message, reader_handle);

        Ok(rx)
    }

    fn get_raw(&mut self) -> Result<UnboundedReceiver<Tick>, String> {
        let (tx, rx) = unbounded();

        let input_channel = self.channel.clone();
        thread::spawn(move || {
            let in_rx = sub_channel(CONF.redis_host, input_channel.as_str());

            for t_string in in_rx.wait() {
                let t = Tick::from_json_string(t_string.unwrap());
                tx.send(t).unwrap();
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
