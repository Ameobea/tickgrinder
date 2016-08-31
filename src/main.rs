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
use transport::command_server::CommandServer;
use algobot_util::transport::commands::*;

use conf::CONF;

fn main() {
    let mut command_server = CommandServer::new(CONF.conn_senders);
    let prom = command_server.execute(Command::AddSMA{period: 5.2342f64});
    println!("{:?}", prom.wait());
    // prom.and_then(|res| {
    //     println!("Result of command: {:?}", res);
    //     Ok(())
    // });
    loop {
        thread::sleep(Duration::new(1, 0));
    }
}
