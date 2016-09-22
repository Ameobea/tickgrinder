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
extern crate uuid;
extern crate algobot_util;

mod datafield;
mod calc;
mod transport;
mod conf;
mod processor;
mod tests;

use std::thread;
use std::time::Duration;
use std::str::FromStr;

use futures::Future;
use futures::stream::{Stream, MergedItem};

use processor::Processor;
use conf::CONF;
use algobot_util::tick::{SymbolTick};
use algobot_util::transport::postgres::{get_client, reset_db, PostgresConf};
use algobot_util::transport::redis::sub_channel;

fn handle_messages() {
    // subscribe to live ticks channel
    let ticks_rx = sub_channel(CONF.redis_url, CONF.redis_ticks_channel);

    let mut processor = Processor::new(String::from_str(CONF.symbol).unwrap());
    // listen for new commands
    let cmds_rx = sub_channel(CONF.redis_url, CONF.redis_control_channel);

    ticks_rx.merge(cmds_rx).for_each(move |mi| {
        match mi {
            MergedItem::First(raw_string) =>
                processor.process(SymbolTick::from_json_string(raw_string)),
            MergedItem::Second(raw_string) =>
                processor.execute_command(raw_string),
            MergedItem::Both(_, _) => ()
        };
        Ok(())
    }).forget();
}

fn main() {
    if CONF.reset_db_on_load {
        let pg_conf = PostgresConf {
            postgres_user: CONF.postgres_user,
            postgres_password: CONF.postgres_password,
            postgres_url: CONF.postgres_url,
            postgres_port: CONF.postgres_port,
            postgres_db: CONF.postgres_db
        };
        reset_db(&get_client(pg_conf).expect("Unable to get postgres client"), CONF.postgres_user)
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
