// Algobot 4
// Casey Primozic, 2016-2016

#![feature(custom_derive, plugin, test)]
#![plugin(serde_macros)]
#![allow(dead_code)]

extern crate redis;
extern crate futures;
extern crate serde_json;
extern crate postgres;
extern crate test;

mod datafield;
mod calc;
mod tick;
mod transport;
mod conf;
mod processor;
mod tests;

use std::thread;
use std::time::Duration;
use std::error::Error;

use futures::*;
use futures::stream::{Stream, Receiver};

use tick::Tick;
use transport::redis::sub_channel;
use transport::postgres::{get_client, reset_db};
use processor::Processor;
use conf::CONF;

fn handle_ticks(rx: Receiver<String, ()>) {
    let mut processor: Processor = Processor::new();
    // do something each time something is received on the Receiver
    rx.for_each(move |res| {
        let mut processor = &mut processor;
        match Tick::from_json_string(res) {
            Ok(t) => processor.process(t),
            Err(e) => println!("{:?}", e.description()),
        }
        Ok(())
    }).forget(); // register this callback and continue program's execution
}

fn main() {
    if CONF.reset_db_on_load {
        reset_db(&get_client().expect("Unable to get postgres client"))
            .expect("Unable to reset database");
        println!("Database reset");
    }

    // rx returns (payload: String, channel_n)
    let rx = sub_channel(CONF.redis_ticks_channel);
    handle_ticks(rx);

    loop {
        // keep program alive but don't swamp the CPU
        thread::sleep(Duration::new(500, 0));
    }
}
