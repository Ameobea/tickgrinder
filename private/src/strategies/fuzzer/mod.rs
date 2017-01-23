//! Broker fuzzer.  See README.md for a full description.

use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use libc::c_void;
use rand::{self, Rng};

use futures::{Future, Stream, Sink, Complete};
use futures::sync::mpsc::{unbounded, Sender};

use tickgrinder_util::strategies::Strategy;
use tickgrinder_util::trading::broker::{Broker, BrokerResult};
use tickgrinder_util::trading::objects::BrokerAction;
use tickgrinder_util::trading::tick::Tick;
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::query_server::QueryServer;
use tickgrinder_util::transport::textlog::get_logger_handle;
use tickgrinder_util::conf::CONF;

// link with the libboost_random wrapper
#[link(name="rand_bindings")]
extern {
    fn init_rng(seed: u32) -> *mut c_void;
    fn rand_int_range(void_rng: *mut c_void, min: i32, max: i32) -> u32;
}

pub struct Fuzzer {
    pub gen: *mut c_void,
    pub logger: EventLogger,
    pub pairs: Vec<String>,
}

impl Strategy for Fuzzer {
    fn new(cs: CommandServer, qs: QueryServer, conf: HashMap<String, String>) -> Fuzzer {
        // convert the seed string into an integer we can use to seen the PNRG if deterministic fuzzing is enabled
        let seed: u32 = if CONF.fuzzer_deterministic_rng {
            let mut sum = 0;
            // convert the seed string into an integer for seeding the fuzzer
            for c in CONF.fuzzer_seed.chars() {
                sum += c as u32;
            }
            sum
        } else {
            let mut rng = rand::thread_rng();
            rng.gen()
        };

        // parse the settings HashMap to get the list of pairs to subscribe to
        let pairs_list = conf.get("pairs")
            .expect("This needs a list of pairs to subscribe to from the connected broker, else we can't do anything!");
        let pairs: Vec<String> = (&pairs_list).split(", ").map(|x| String::from(x)).collect();

        Fuzzer {
            gen: unsafe { init_rng(seed)},
            logger: EventLogger::new(),
            pairs: pairs,
        }
    }

    fn init(&mut self, mut broker: Box<Broker>) {
        // subscribe to all the tickstreams as supplied in the configuration and combine the streams
        let (streams_tx, streams_rx) = unbounded();
        let mut symbol_enumeration = Vec::new(); // way to match symbols with their id
        for (i, symbol) in (&self.pairs).iter().enumerate() {
            let streams_tx = &streams_tx;
            symbol_enumeration.push((i, symbol,));
            let rx = broker.sub_ticks(symbol.clone())
                .expect(&format!("Unable to sub ticks for symbol {}", symbol))
                .map(move |t| (i, t));
            streams_tx.send(rx).unwrap();
        }
        let master_rx = streams_rx.flatten();
        self.logger.log_misc(format!("Subscribed to {} tickstreams", symbol_enumeration.len()));

        // start responding to ticks from all the streams.
        self.logger.log_misc(String::from("Initializing fuzzer loop..."));
        for msg in master_rx.wait() {
            let (i, t) = msg.unwrap();
            self.logger.log_tick(t, i);

            match get_action(t, self.gen) {
                Some(action) => {
                    let id = self.logger.log_request(&action);
                    let fut = broker.execute(action);
                    let res = fut.wait().unwrap();
                    self.logger.log_response(&res, id);
                },
                None => (),
            }
        }
    }

    fn exit_now(&mut self, ready: Complete<()>) {
        unimplemented!();
    }
}

/// Called during each iteration of the fuzzer loop.  Picks a random action to take based on the
/// internally held PRNG and executes it.
pub fn get_action(t: Tick, gen: *mut c_void) -> Option<BrokerAction> {
    let rand = unsafe { rand_int_range(gen, 0, 5) };
    let action_opt: Option<BrokerAction> = match rand {
        0 => Some(BrokerAction::Ping),
        1 => None, // TODO
        // sleep for a few milliseconds, then do either do nothing or perform an action
        2 => {
            let sleep_time = unsafe { rand_int_range(gen, 0, 25) };
            thread::sleep(Duration::from_millis(sleep_time as u64));
            let action_or_no = unsafe { rand_int_range(gen, 0, 5) };
            if action_or_no > 3 {
                get_action(t, gen)
            } else {
                None
            }
        },
        // do nothing at all in response to the tick
        _ => None,
    };

    action_opt
}

pub struct EventLogger {
    tx: Option<Sender<String>>,
    i: usize, // incremented every time an event is logged in order to record order
}

impl EventLogger {
    /// Initializes a new logger thread and returns handle to it
    /// TODO: write header info into the log file about symbol/symbol_id pairing etc.
    pub fn new() -> EventLogger {
        let tx = get_logger_handle(String::from("fuzzer"), 50);

        EventLogger {
            tx: Some(tx),
            i: 0,
        }
    }

    /// Logs an event taking place during the fuzzing process.  Returns a number to be used to match
    /// the request to a response.
    pub fn log_request(&mut self, action: &BrokerAction) -> usize {
        println!("Sending request to broker: {:?}", action);
        self.i += 1;
        let tx = self.tx.take().unwrap();
        let new_tx = tx.send(format!("{} -  REQUEST: {:?}", self.i, action))
            .wait().expect("Unable to log request!");
        self.tx = Some(new_tx);
        self.i
    }

    /// Logs a response received from the broker
    pub fn log_response(&mut self, res: &BrokerResult, id: usize) {
        println!("Got response from broker: {:?}", res);
        let tx = self.tx.take().unwrap();
        let new_tx = tx.send(format!("{} - RESPONSE: {:?}", id, res))
            .wait().expect("Unable to log response!");
        self.tx = Some(new_tx);
    }

    /// Logs the fuzzer receiving a tick from the broker.  `i` is the index of that symbol.
    pub fn log_tick(&mut self, t: Tick, i: usize) {
        println!("Received new tick from broker: {:?}", t);
        let tx = self.tx.take().unwrap();
        let new_tx = tx.send(format!("Received tick from symbol with index {}: {:?}", i, t))
            .wait().expect("Unable to log tick!");
        self.tx = Some(new_tx);
    }

    /// Logs a plain old text message
    pub fn log_misc(&mut self, msg: String) {
        println!("Message: {}", msg);
        let tx = self.tx.take().unwrap();
        let new_tx = tx.send(msg).wait().expect("Logging tick failed");
        self.tx = Some(new_tx);
    }
}

// Make sure that the values we pull out of the seeded random number generator really are deterministic.
#[test]
fn deterministic_rng() {
    unsafe {
        let gen1 = init_rng(12345u32);
        let gen2 = init_rng(12345u32);

        let rand1 = rand_int_range(gen1, 1i32, 1000000i32);
        let rand2 = rand_int_range(gen2, 1i32, 1000000i32);

        assert_eq!(rand1, rand2);
    }
}
