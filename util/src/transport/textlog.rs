//! Helper functions for logging to file.

use std::thread;
use std::path::PathBuf;
use std::fs::{DirBuilder, File};
use std::io::Write;
use std::fmt::Debug;
use time::now;

use futures::Stream;
use futures::sync::mpsc::{channel, Sender};

use conf::CONF;

/// Returns a `Sender` that writes all `String`s sent to it to file.  `dirname` is the name of the subdirectory of the
/// `[CONF.data_dir]/logs` directory where the file is written and `chunk_size` is how many lines are stored in memory
/// before being written to disc.
pub fn get_logger_handle(dirname: String, chunk_size: usize) -> Sender<String> {
    let (tx, rx) = channel(chunk_size);

    // spawn the logger thread and initialize the logging loop
    thread::spawn(move || {
        let dirname = &dirname;
        // if the directories don't exist in the logging directory, create them
        let log_dir: PathBuf = PathBuf::from(CONF.data_dir).join("logs").join(dirname);
        if !log_dir.is_dir() {
            let mut builder = DirBuilder::new();
            builder.recursive(true).create(log_dir.clone())
                .expect("Unable to create directory to hold the log files; permission issue or bad data dir configured?");
        }

        println!("Attempting to find valid filename...");
        let mut attempts = 1;
        let curtime = now();
        let mut datestring = format!("{}-{}_{}.log", curtime.tm_mon + 1, curtime.tm_mday, attempts);
        while PathBuf::from(CONF.data_dir).join("logs").join(dirname).join(&datestring).exists() {
            attempts += 1;
            datestring = format!("{}-{}_{}.log", curtime.tm_mon + 1, curtime.tm_mday, attempts);
        }

        println!("creating log file...");
        let datestring = format!("{}-{}_{}.log", curtime.tm_mon + 1, curtime.tm_mday, attempts);
        let mut file = File::create(PathBuf::from(CONF.data_dir).join("logs").join(dirname).join(&datestring))
            .expect("Unable to create log file!");

        // buffer up chunk_size log lines before writing to disk
        for msg in rx.chunks(chunk_size).wait() {
            // println!("Logging message chunk...");
            let text: String = match msg {
                Ok(lines) => lines.as_slice().join("\n") + "\n",
                // World is likely dropping due to a crash or shutdown
                Err(_) => unimplemented!(),
            };

            // write the chunk_size lines into the file
            write!(&mut file, "{}", text).expect("Unable to write lines into log file!");
        }
    });

    tx
}

/// Given a type that can be debug-formatted, returns a String that contains its debug-formatted version.
pub fn debug<T>(x: T) -> String where T:Debug {
    format!("{:?}", x)
}
