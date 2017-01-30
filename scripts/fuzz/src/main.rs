extern crate tickgrinder_util;
extern crate simbroker;
extern crate private;
extern crate uuid;
extern crate futures;

use std::collections::HashMap;

use uuid::Uuid;
use futures::{Future, Stream};
use futures::sync::mpsc::unbounded;

use tickgrinder_util::trading::broker::Broker;
use tickgrinder_util::strategies::{Strategy, ManagedStrategy, StrategyManager, Helper};
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::query_server::QueryServer;
use simbroker::SimBrokerClient;
use private::strategies::fuzzer::Fuzzer;

/// Consumes all the tickstreams and routes them into the fuzzer.
struct FuzzerExecutor {}

impl FuzzerExecutor {
    fn exec(self, mut manager: StrategyManager, pairs: &[String]) {
        // subscribe to all the tickstreams as supplied in the configuration and combine the streams
        let (streams_tx, streams_rx) = unbounded();
        let mut symbol_enumeration = Vec::new(); // way to match symbols with their id
        for (i, symbol) in pairs.iter().enumerate() {
            let streams_tx = &streams_tx;
            symbol_enumeration.push((i, symbol,));
            let rx = manager.helper.broker.sub_ticks(symbol.clone())
                .expect(&format!("Unable to sub ticks for symbol {}", symbol))
                .map(move |t| (i, t));
            streams_tx.send(rx).unwrap();
        }
        let master_rx = streams_rx.flatten();
        manager.helper.cs.notice(None, &format!("Subscribed to {} tickstreams", symbol_enumeration.len()));

        // block this thread and funnel all messages from the tickstreams into the fuzzer
        for msg in master_rx.wait() {
            let (i, t) = msg.expect("Message was `Err` in `master_rx`!");
            manager.tick(i, &t);
        }
    }
}

fn main() {
    let client = Box::new(SimBrokerClient::init(HashMap::new()).wait().unwrap().unwrap());
    let mut hm = HashMap::new();
    hm.insert(String::from("pairs"), String::from("TEST"));
    let mut fuzzer = Fuzzer::new(hm.clone());

    // create a strategy manager to manage the fuzzer and initialize it
    let mut manager = StrategyManager::new(Box::new(fuzzer), client);
    manager.init();

    // create a strategy executor for the fuzzer and initialize it to start the fuzzing process
    let executor = FuzzerExecutor{};
    executor.exec(manager, &[String::from("TEST")]);
}
