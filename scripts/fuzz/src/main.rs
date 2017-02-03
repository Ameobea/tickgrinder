extern crate tickgrinder_util;
extern crate simbroker;
extern crate private;
extern crate uuid;
extern crate futures;

use std::collections::HashMap;

use futures::{Future, Stream, Sink};
use futures::sync::oneshot;
use futures::sync::mpsc::{unbounded, Sender};
use futures::stream::MergedItem;

use tickgrinder_util::trading::broker::{Broker, BrokerResult};
use tickgrinder_util::strategies::{Strategy, StrategyManager, StrategyAction};

use simbroker::SimBrokerClient;
use private::strategies::fuzzer::Fuzzer;

/// Consumes all the tickstreams and routes them into the fuzzer.
struct SimbrokerDriver {}

impl SimbrokerDriver {
    pub fn exec(
        self, mut manager: StrategyManager<()>, pairs: &[String], mut bufstream_tx: Sender<oneshot::Receiver<BrokerResult>>
    ) {
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
        let tick_rx = streams_rx.flatten();
        let push_rx = manager.helper.broker.get_stream().unwrap();
        let mut merged_rx_iter = tick_rx.merge(push_rx).wait();
        manager.helper.cs.notice(None, &format!("Subscribed to {} tickstreams", symbol_enumeration.len()));

        // start the simulation loop off to populate the tickstreams
        manager.helper.broker.send_message(0);

        // block this thread and funnel all messages from the tickstreams into the fuzzer
        let mut client_msg_count; // how many notifications from the simbroker this tick
        let mut client_res_count = 0; // how many respones we're sending back in response to this tick
        loop {
            // manually drive progress on the inner event loop by abusing our custom message functionality
            client_msg_count = manager.helper.broker.send_message(client_res_count);
            client_res_count = 0;

            for _ in 0..client_msg_count {
                let response = match merged_rx_iter.next().unwrap().unwrap() {
                    MergedItem::First((ix, tick)) => manager.broker_tick(ix, tick),
                    MergedItem::Second((timestamp, res)) => manager.pushstream_tick(res, timestamp),
                    MergedItem::Both((_, _), (_, _)) => panic!("Received both in simbroker driver."),
                };

                let (new_bufstream_tx, responses) = SimbrokerDriver::handle_response(response, &mut manager, bufstream_tx);
                bufstream_tx = new_bufstream_tx;
                client_res_count += responses;
            }
        }
    }

    /// Handles the `StrategyAction` returned by the strategy (if there is one) and returns the number
    /// of actions that were processed and a new `bufstream_tx` (since it's consumed during `send()`).
    fn handle_response(
        response: Option<StrategyAction>, manager: &mut StrategyManager<()>, bufstream_tx: Sender<oneshot::Receiver<BrokerResult>>
    ) -> (Sender<oneshot::Receiver<BrokerResult>>, usize) {
        // for now, only one command is returned by strategies every tick
        let responses = if response.is_none() { 0 } else { 1 };
        let new_bufstream_tx;

        match response {
            Some(strat_action) => {
                match strat_action {
                    StrategyAction::BrokerAction(broker_action) => {
                        // println!("`execute()` on broker...");
                        let fut = manager.helper.broker.execute(broker_action);
                        // println!("`bufstream_tx.send()`...");
                        new_bufstream_tx = bufstream_tx.send(fut).wait().unwrap();
                        // println!("After `bufstream_tx.send()`.");
                    },
                     _ => unimplemented!(),
                }
            },
            None => new_bufstream_tx = bufstream_tx,
        };

        (new_bufstream_tx, responses)
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
    let mut manager = StrategyManager::new(Box::new(fuzzer), client, Vec::new());
    manager.init();

    // create a strategy executor for the fuzzer and initialize it to start the fuzzing process
    let executor = SimbrokerDriver{};
    executor.exec(manager, &[String::from("TEST")], bufstream_tx);
}
