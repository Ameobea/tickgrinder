//! A TickGenerator that reads historical ticks out of CSV files.

use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::thread;

use futures::Future;
use futures::stream::{channel, Receiver};
use algobot_util::trading::tick::Tick;

use data::*;
use conf::CONF;

pub struct FlatfileReader {}

impl TickGenerator for FlatfileReader {
    /// Returns a result that yeilds a Stream of Results if the source
    /// is available and a which yeild Ticks if the file is formatted correctly.
    fn get(&mut self, symbol: String) -> Result<Receiver<Tick, ()>, String> {
        let mut path = PathBuf::from(CONF.tick_data_dir);
        let filename = format!("{}.csv", symbol);
        path.push(filename.as_str());

        let (mut sender, receiver) = channel::<Tick, ()>();
        let file_opt = File::open(path);
        let file = match file_opt {
            Ok(file) => file,
            Err(e) => return Err(e.to_string())
        };
        let mut buf_reader = BufReader::new(file);

        thread::spawn(move || {
            loop {
                let mut line = String::new();
                let n = buf_reader.read_line(&mut line).unwrap();
                if n == 0 {
                    break;
                }
                let t = Tick::from_csv_string(line.as_str());
                sender = sender.send(Ok(t)).wait().ok().unwrap();
            }
            println!("No more lines to send.");
        });

        Ok(receiver)
    }

    fn get_name(&self) -> &'static str {
        "Flatfile"
    }
}
