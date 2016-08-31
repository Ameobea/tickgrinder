//! Algobot 4 Optimizer
//! Created by Casey Primozic 2016-2016

#![allow(unconditional_recursion)]
#![feature(conservative_impl_trait, custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate uuid;
extern crate postgres;
extern crate redis;
extern crate futures;
extern crate serde;
extern crate serde_json;

extern crate algobot_util;

mod transport;
mod conf;

use std::thread;
use std::time::Duration;

use futures::Future;

use transport::command_server::*;
use conf::CONF;

fn main() {
    let mut command_server = CommandServer::new(CONF.conn_senders);
    let prom = command_server.execute(Command::Ping);
    println!("{:?}", prom.wait());
    // prom.and_then(|res| {
    //     println!("Result of command: {:?}", res);
    //     Ok(())
    // });
    loop {
        let mut command_server = &mut command_server;
        thread::sleep(Duration::new(1, 0));
    }
}
