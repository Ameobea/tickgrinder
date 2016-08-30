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

use transport::command_server::CommandServer;
use conf::CONF;

fn main() {
    let command_server = CommandServer::new(CONF.conn_senders);
    thread::sleep(Duration::new(50000, 0));
}
