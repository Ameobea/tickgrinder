// Algobot 3, Rust Version
// Casey Primozic, 2016-2016

#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate redis;
extern crate futures;
extern crate serde_json;

mod datafield;
mod calc;
mod tick;
mod transport;
mod conf;
mod processor;

use std::thread;
use std::time::Duration;
use std::error::Error;

use futures::*;
use futures::stream::{Stream, Sender, Receiver, channel};

use tick::Tick;
use transport::Tickstream;
use processor::Processor;

// create a thread that listens for new messages on redis
// and resets itself after the results are consumed
fn get_ticks(tx: Sender<String, ()>) {
    thread::spawn(move || {
        get_ticks_inner(tx, Tickstream::new())
    });
}

fn get_ticks_inner(tx: Sender<String, ()>, mut ts: Tickstream) {
    // perform blocking fetch operation inside the thread
    let res = ts.get_tick();
    // send the result from redis through the channel,
    // which returns a new tx.
    tx.send(Ok(res)).and_then(|new_tx| {
        // call this function and try to fetch another tick.
        get_ticks_inner(new_tx, ts);
        Ok(())
    }).forget();
}

fn handle_ticks(rx: Receiver<String, ()>) {
    let mut processor: Processor = Processor::new();
    // do something each time something is received on the Receiver
    rx.for_each(move |res| {
        let mut processor = &mut processor;
        match Tick::from_string(res) {
            Ok(t) => processor.process(t),
            Err(e) => println!("{:?}", e.description()),
        }
        Ok(())
    }).forget(); // register this callback and continue program's execution
}

fn main() {
    let (tx, rx) = channel::<String, ()>();
    // start listening for new ticks on a separate thread
    get_ticks(tx);
    handle_ticks(rx);

    loop {
        // keep program alive but don't swamp the CPU
        thread::sleep(Duration::new(500, 0));
    }
}
