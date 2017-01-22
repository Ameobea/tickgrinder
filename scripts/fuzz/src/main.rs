extern crate tickgrinder_util;
extern crate simbroker;
extern crate private;
extern crate uuid;

use std::collections::HashMap;

use uuid::Uuid;

use tickgrinder_util::trading::broker::Broker;
use tickgrinder_util::strategies::Strategy;
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::query_server::QueryServer;
use simbroker::SimBrokerClient;
use private::strategies::fuzzer::Fuzzer;

fn main() {
    let client = SimBrokerClient::init(HashMap::new());
    let fuzzer = Fuzzer::new(CommandServer::new(Uuid::new_v4(), "Fuzzer"), QueryServer::new(2), HashMap::new());
}
