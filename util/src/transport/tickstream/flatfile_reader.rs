//! A `TickGenerator` that reads historical ticks out of CSV files.

use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::thread;


use futures::sync::mpsc::{unbounded, UnboundedReceiver};
use trading::tick::Tick;
use conf::CONF;

use super::*;

pub struct FlatfileReader {
    pub symbol: String,
    pub start_time: Option<u64>,
}

impl TickGenerator for FlatfileReader {
    fn get(
        &mut self, mut map: Box<TickMap + Send>, cmd_handle: CommandStream
    )-> Result<UnboundedReceiver<Tick>, String> {
        // small atomic communication bus between the handle listener and worker threads
        let internal_message: Arc<Mutex<TickstreamCommand>> = Arc::new(Mutex::new(TickstreamCommand::Stop));
        let got_mail = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = unbounded::<Tick>();

        // spawn the worker thread that does the blocking
        let mut _got_mail = got_mail.clone();
        let _internal_message = internal_message.clone();
        let symbol = self.symbol.clone();
        let start_time = self.start_time;
        let reader_handle = thread::spawn(move || {
            // open the file and get an iterator over its lines set to the starting point
            let iter_ = init_reader(&symbol);
            if iter_.is_err() {
                println!("Unable to open the file!");
            }
            let iter = iter_.unwrap().skip_while(|t| {
                start_time.is_some() && t.timestamp < start_time.unwrap()
            });

            for tick in iter {
                if check_mail(&*got_mail, &*_internal_message) {
                    println!("Stop command received; killing reader");
                    break;
                }

                // apply the map
                let t_mod = map.map(tick);
                if t_mod.is_some() {
                    sender.send(tick).unwrap();
                }
            }
        }).thread().clone();

        // spawn the handle listener thread that awaits commands
        spawn_listener_thread(_got_mail, cmd_handle, internal_message, reader_handle);

        Ok(receiver)
    }

    fn get_raw(&mut self) -> Result<UnboundedReceiver<Tick>, String> {
        let (tx, rx) = unbounded();

        let start_time = self.start_time;
        let symbol = self.symbol.clone();
        thread::spawn(move || {
            let iter_ = init_reader(&symbol);
            if iter_.is_err() {
                println!("Unable to open the file!");
            }
            let iter = iter_.unwrap().skip_while(|t| {
                start_time.is_some() && t.timestamp < start_time.unwrap()
            });

            for tick in iter {
                tx.send(tick).unwrap();
            }
        });

        Ok(rx)
    }
}

/// Trys to open the file containing the historical ticks for the supplied symbol.
pub fn init_reader(symbol: &str) -> Result<impl Iterator<Item=Tick>, String> {
    let mut path = PathBuf::from(CONF.data_dir);
    path.push("historical_ticks");
    let filename = format!("{}.csv", symbol.to_uppercase());
    path.push(filename.as_str());

    let file = try!(File::open(path).map_err( |e| e.to_string() ));
    Ok(BufReader::new(file).lines().map( |line| {
        Tick::from_csv_string(line.unwrap().as_str())
    }))
}

