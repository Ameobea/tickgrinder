extern crate tickgrinder_util;
extern crate simbroker;
extern crate private;
extern crate uuid;
extern crate futures;

use std::collections::HashMap;

use futures::{Future, Sink};
use futures::sync::oneshot;
use futures::sync::mpsc::Sender;

use tickgrinder_util::trading::broker::{Broker, BrokerResult};
use tickgrinder_util::trading::tick::Tick;
use tickgrinder_util::strategies::{Strategy, StrategyManager, StrategyAction};

use simbroker::{SimBrokerClient, TickOutput};
use private::strategies::fuzzer::Fuzzer;

/// Consumes all the tickstreams and routes them into the fuzzer.
struct SimbrokerDriver {}

impl SimbrokerDriver {
    pub fn exec(
        self, mut manager: StrategyManager<SimBrokerClient, ()>, _: &[String],
        mut bufstream_tx: Sender<oneshot::Receiver<BrokerResult>>
    ) {
        // block this thread and funnel all messages from the tickstreams into the fuzzer
        let mut client_msg_count; // how many notifications from the simbroker this tick
        let mut client_res_count = 0; // how many respones we're sending back in response to this tick
        let mut buffer = Vec::new(); // passed to the simbroker to be filled with responses
        buffer.resize(420, TickOutput::Tick(99, Tick::null())); // full the buffer with dummy values
        loop {
            // manually drive progress on the inner event loop by abusing our custom message functionality
            client_msg_count = manager.helper.broker.tick_sim_loop(client_res_count, &mut buffer);
            client_res_count = 0;

            for i in 0..client_msg_count {
                let response = match &buffer[i] {
                    &TickOutput::Tick(ix, tick) => manager.broker_tick(ix, tick),
                    &TickOutput::Pushstream(timestamp, ref res) => manager.pushstream_tick(res.clone(), timestamp),
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
        response: Option<StrategyAction>, manager: &mut StrategyManager<SimBrokerClient, ()>,
        bufstream_tx: Sender<oneshot::Receiver<BrokerResult>>
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
    let client = SimBrokerClient::init(HashMap::new()).wait().unwrap().unwrap();
    let mut hm = HashMap::new();
    hm.insert(String::from("pairs"), String::from("TEST"));

    // create a Fuzzer instance and grab some internals to use here
    let mut fuzzer = Fuzzer::new(hm.clone());
    let bufstream_tx = fuzzer.events_tx.take().unwrap();

    // create a strategy manager to manage the fuzzer and initialize it, then initialize the simbroker simulation loop
    let mut manager: StrategyManager<SimBrokerClient, ()> = StrategyManager::new(Box::new(fuzzer), client, Vec::new());
    manager.init();
    manager.helper.broker.init_sim_loop().expect("Unable to initialize sim loop");

    // create a strategy executor for the fuzzer and initialize it to start the fuzzing process
    let executor = SimbrokerDriver{};
    executor.exec(manager, &[String::from("TEST")], bufstream_tx);
}
