//! A TickGenerator that reads historical ticks out of CSV files.

use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

use futures::Future;
use futures::stream::{channel, Receiver};
use algobot_util::trading::tick::Tick;

use data::*;
use backtest::{BacktestCommand, BacktestMap};
use conf::CONF;

pub struct FlatfileReader {
    pub symbol: String
}

impl TickGenerator for FlatfileReader {
    fn get(
        &mut self, mut map: Box<BacktestMap + Send>, cmd_handle: CommandStream
    )-> Result<Receiver<Tick, ()>, String> {
        let mut path = PathBuf::from(CONF.tick_data_dir);
        let filename = format!("{}.csv", self.symbol);
        path.push(filename.as_str());

        let (mut sender, receiver) = channel::<Tick, ()>();
        let file_opt = File::open(path);
        let file = match file_opt {
            Ok(file) => file,
            Err(e) => return Err(e.to_string())
        };
        let mut buf_reader = BufReader::new(file);

        // small atomic communication bus between the handle listener and worker threads
        let internal_message: Arc<Mutex<BacktestCommand>> = Arc::new(Mutex::new(BacktestCommand::Stop));
        let _internal_message = internal_message.clone();
        let got_mail = Arc::new(AtomicBool::new(false));
        let mut _got_mail = got_mail.clone();

        // spawn the worker thread that does the blocking
        let reader_handle = thread::spawn(move || {
            let cur_command: Option<BacktestCommand> = None;
            loop {
                if check_mail(&*got_mail, &*_internal_message) {
                    println!("Stop command received; killing reader");
                    break;
                }

                let mut line = String::new();
                let n = buf_reader.read_line(&mut line).unwrap();
                if n == 0 {
                    break;
                }
                let t = Tick::from_csv_string(line.as_str());

                // apply the map
                let t_mod = map.map(t);
                if t_mod.is_some() {
                    sender = sender.send(Ok(t)).wait().ok().unwrap();
                }
            }
        }).thread().clone();

        // spawn the handle listener thread that awaits commands
        spawn_listener_thread(_got_mail, cmd_handle, internal_message, reader_handle);

        Ok(receiver)
    }
}
