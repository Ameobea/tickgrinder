// Algobot 3, Rust Version
// Casey Primozic, 2016-2016

extern crate redis;
extern crate futures;
// extern crate futures_cpupool;

use std::thread;
use std::time::Duration;

use futures::*;
use futures::stream::{Stream, Sender, Receiver, channel};

mod datafield;
mod calc;
mod tick;
mod transport;
mod conf;

use transport::Tickstream;

// create a thread that listens for new messages on redis
// and resets itself after the results are consumed
fn get_ticks(tx: Sender<String, ()>) {
    let mut ts = Tickstream::new();
    let listener = thread::spawn(move || {
        // perform blocking read operation inside the thread
        let res = ts.get_tick();
        // send the result from redis through the channel,
        // which returns a new tx.
        let new_tx = tx.send(Ok(res)).and_then(|new_tx| {
            println!("{:?}", "got it");
            // call this function and restart the process of listening for ticks.
            get_ticks(new_tx);
            Ok(())
        }).forget();
    });
}

fn main() {
    let (tx, rx) = channel::<String, ()>();

    // start listening for new ticks on a separate thread
    get_ticks(tx);

    // do something each time something is received on the Receiver
    rx.for_each(|res| {
        // do whatever you want to do with the received tick.
        // This will be handled asynchronously.
        println!("{:?}", res);
        Ok(())
    }).forget(); // register this callback and continue program's execution

    loop {
        // do whatever you want here and the tick listener
        // and callback will continue to run in the background
        thread::sleep(Duration::new(5, 0));
    }
}
