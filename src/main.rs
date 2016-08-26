// Algobot 4
// Casey Primozic, 2016-2016


#![feature(custom_derive, plugin, test, conservative_impl_trait)]
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

use futures::stream::{Stream, MergedItem};

use tick::Tick;
use transport::redis::sub_channel;
use transport::postgres::{get_client, reset_db};
use processor::Processor;
use conf::CONF;

fn handle_messages() {
    // subscribe to live ticks channel
    let ticks_rx = sub_channel(CONF.redis_ticks_channel);

    let mut processor = Processor::new(CONF.symbol);
    // listen for new commands
    let cmds_rx = sub_channel(CONF.redis_control_channel);

    ticks_rx.merge(cmds_rx).for_each(move |mi| {
        match mi {
            MergedItem::First(raw_string) =>
                processor.process(Tick::from_json_string(raw_string)),
            MergedItem::Second(raw_string) =>
                processor.execute_command(raw_string),
            MergedItem::Both(_, _) => ()
        };
        Ok(())
    });
}

fn main() {
    if CONF.reset_db_on_load {
        reset_db(&get_client().expect("Unable to get postgres client"))
            .expect("Unable to reset database");
        println!("Database reset");
    }

    // Start the listeners for everything
    handle_messages();

    loop {
        // keep program alive but don't swamp the CPU
        thread::sleep(Duration::new(500, 0));
    }
}
