//! Supplies the backtester with historical ticks stored in a variety of formats.

use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::thread;

use futures::Future;
use futures::stream::{channel, Stream, Receiver};

use conf::CONF;
use algobot_util::tick::*;

/// Any object that is capable of providing a stream of historical ticks
pub trait TickReader {
    /// Returns a stream that resolves to new Ticks
    fn get(symbol: String) -> Result<Receiver<Tick, ()>, String>;
}

pub struct FlatfileReader {}

impl TickReader for FlatfileReader {
    /// Returns a result that yeilds a Stream of Results if the source
    /// is available and a which yeild Ticks if the file is formatted correctly.
    fn get(symbol: String) -> Result<Receiver<Tick, ()>, String> {
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
}
