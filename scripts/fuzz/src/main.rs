extern crate tickgrinder_util;
extern crate simbroker;
extern crate private;
extern crate uuid;
extern crate futures;

use std::collections::HashMap;

use uuid::Uuid;
use futures::Future;

use tickgrinder_util::trading::broker::Broker;
use tickgrinder_util::strategies::Strategy;
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::query_server::QueryServer;
use simbroker::SimBrokerClient;
use private::strategies::fuzzer::Fuzzer;

fn main() {
    let client = Box::new(SimBrokerClient::init(HashMap::new()).wait().unwrap().unwrap());
    let mut hm = HashMap::new();
    hm.insert(String::from("pairs"), String::from("TEST"));
    let mut fuzzer = Fuzzer::new(CommandServer::new(Uuid::new_v4(), "Fuzzer"), QueryServer::new(2), hm);

    fuzzer.init(client);
}
