extern crate tickgrinder_util;
extern crate simbroker;
extern crate private;
extern crate uuid;
extern crate futures;

use std::collections::HashMap;

use futures::{Future, Stream, Sink};
use futures::sync::oneshot;
use futures::sync::mpsc::{unbounded, Sender};

use tickgrinder_util::trading::broker::{Broker, BrokerResult};
use tickgrinder_util::strategies::{Strategy, StrategyManager, StrategyAction};

use simbroker::SimBrokerClient;
use private::strategies::fuzzer::Fuzzer;

/// Consumes all the tickstreams and routes them into the fuzzer.
struct FuzzerExecutor {}

impl FuzzerExecutor {
    fn exec(self, mut manager: StrategyManager, pairs: &[String], mut bufstream_tx: Sender<oneshot::Receiver<BrokerResult>>) {
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
        let mut master_rx_iter = streams_rx.flatten().wait();
        manager.helper.cs.notice(None, &format!("Subscribed to {} tickstreams", symbol_enumeration.len()));

        // start the simulation loop off to populate the tickstreams
        manager.helper.broker.send_message(0);

        // block this thread and funnel all messages from the tickstreams into the fuzzer
        loop {
            let (i, t) = master_rx_iter.next().unwrap().unwrap();
            let response = manager.tick(i, &t);
            // for now, only one command is returned by strategies every tick
            let responses = if response.is_none() { 0 } else { 1 };

            match response {
                Some(strat_action) => {
                    match strat_action {
                        StrategyAction::BrokerAction(broker_action) => {
                            println!("`execute()` on broker...");
                            let fut = manager.helper.broker.execute(broker_action);
                            println!("`bufstream_tx.send()`...");
                            bufstream_tx = bufstream_tx.send(fut).wait().unwrap();
                            println!("After `bufstream_tx.send()`.");
                        },
                         _ => unimplemented!(),
                    }
                },
                None => (),
            };

            // manually drive progress on the inner event loop by abusing our custom message functionality
            manager.helper.broker.send_message(responses);
        }
    }
}

fn main() {
    let client = Box::new(SimBrokerClient::init(HashMap::new()).wait().unwrap().unwrap());
    let mut hm = HashMap::new();
    hm.insert(String::from("pairs"), String::from("TEST"));

    // create a Fuzzer instance and grab some internals to use here
    let mut fuzzer = Fuzzer::new(hm.clone());
    let bufstream_tx = fuzzer.events_tx.take().unwrap();

    // create a strategy manager to manage the fuzzer and initialize it
    let mut manager = StrategyManager::new(Box::new(fuzzer), client);
    manager.init();

    // create a strategy executor for the fuzzer and initialize it to start the fuzzing process
    let executor = FuzzerExecutor{};
    executor.exec(manager, &[String::from("TEST")], bufstream_tx);
}
