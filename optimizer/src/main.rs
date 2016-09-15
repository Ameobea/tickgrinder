//! Algobot 4 Optimizer
//! Created by Casey Primozic 2016-2016

#![allow(unconditional_recursion)]
#![feature(conservative_impl_trait, custom_derive, plugin, test)]
#![plugin(serde_macros)]

extern crate test;
extern crate uuid;
extern crate postgres;
extern crate redis;
extern crate futures;
extern crate serde;
extern crate serde_json;

extern crate algobot_util;

mod transport;
mod conf;
#[allow(unused_imports, dead_code)]
mod tests;

use std::thread;
use std::time::Duration;

use futures::Future;
use algobot_util::transport::command_server::{CommandServer, CsSettings};
use algobot_util::transport::commands::*;

use conf::CONF;

fn main() {
    let settings = CsSettings {
        redis_host: CONF.redis_host,
        redis_channel: CONF.redis_response_channel,
        conn_count: CONF.conn_senders,
        timeout: CONF.cs_timeout,
        max_retries: CONF.cs_max_retries
    };
    let mut command_server = CommandServer::new(settings);

    let mut i = 1;
    loop {
        let mut command_server = &mut command_server;
        thread::sleep(Duration::new(0, 4000000));
        println!("{:?}", i);
        let prom = command_server.execute(Command::Ping);
        prom.and_then(|res| {
            println!("{:?}", res);
            Ok(())
        });
        i+=1;
    }
}
