// Algobot 3, Rust Version
// Casey Primozic, 2016-2016

extern crate redis;
extern crate futures;

mod datafield;
mod calc;
mod tick;
mod transport;
mod conf;
mod processor;

use std::thread;
use std::time::Duration;

use futures::*;
use futures::stream::{Stream, Sender, channel};

use tick::Tick;
use transport::Tickstream;
use datafield::DataField;
use processor::Processor;

// create a thread that listens for new messages on redis
// and resets itself after the results are consumed
fn get_ticks(tx: Sender<String, ()>) {
    let listener = thread::spawn(move || {
        get_ticks_inner(tx, Tickstream::new())
    });
}

fn get_ticks_inner(tx: Sender<String, ()>, mut ts: Tickstream) {
    // perform blocking fetch operation inside the thread
    let res = ts.get_tick();
    // send the result from redis through the channel,
    // which returns a new tx.
    let new_tx = tx.send(Ok(res)).and_then(|new_tx| {
        // call this function and try to fetch another tick.
        get_ticks_inner(new_tx, ts);
        Ok(())
    }).forget();
}

fn main() {
    let tf: DataField<Tick> = DataField::new();
    let (tx, rx) = channel::<String, ()>();
    let mut processor: Processor = Processor::new();

    // start listening for new ticks on a separate thread
    get_ticks(tx);

    // do something each time something is received on the Receiver
    rx.for_each(move |t| {
        processor.process(Tick::null());
        Ok(())
    }).forget(); // register this callback and continue program's execution

    loop {
        // keep program alive but don't swamp the CPU
        thread::sleep(Duration::new(500, 0));
    }
}
