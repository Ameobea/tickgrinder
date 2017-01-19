//! Broker fuzzer.  See README.md for a full description.

use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use libc::c_void;
use rand::{self, Rng};

use futures::{Future, Stream, Sink, Complete};
use futures::sync::mpsc::unbounded;

use tickgrinder_util::strategies::Strategy;
use tickgrinder_util::trading::broker::{Broker, BrokerResult};
use tickgrinder_util::trading::objects::BrokerAction;
use tickgrinder_util::trading::tick::Tick;
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::query_server::QueryServer;
use tickgrinder_util::conf::CONF;

use super::super::ActiveBroker;

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

    fn init(&mut self) {
        // first step is to initialize the connection to the broker
        let mut broker = ActiveBroker::init(HashMap::new()).wait().unwrap().unwrap();

        // subscribe to all the tickstreams as supplied in the configuration and combine the streams
        let (streams_tx, streams_rx) = unbounded();
        for symbol in &self.pairs {
            let streams_tx = &streams_tx;
            let rx = broker.sub_ticks(symbol.clone())
                .expect(&format!("Unable to sub ticks for symbol {}", self.pairs[0]));
            streams_tx.send(rx).unwrap();
        }
        let master_rx = streams_rx.flatten();

        // start responding to ticks from all the streams.
        for t in master_rx.wait() {
            match self.get_action(t.unwrap()) {
                Some(action) => {
                    self.logger.log_request(&action);
                    let fut = broker.execute(action);
                    let res = fut.wait().unwrap();
                    self.logger.log_response(&res);
                },
                None => (),
            }
        }
    }

    fn exit_now(&mut self, ready: Complete<()>) {
        unimplemented!();
    }
}

impl Fuzzer {
    /// Called during each iteration of the fuzzer loop.  Picks a random action to take based on the
    /// internally held PRNG and executes it.
    pub fn get_action(&mut self, t: Tick) -> Option<BrokerAction> {
        let rand = unsafe { rand_int_range(self.gen, 0, 5) };
        let action_opt: Option<BrokerAction> = match rand {
            0 => Some(BrokerAction::Ping),
            1 => unimplemented!(), // TODO
            // sleep for a few milliseconds, then do either do nothing or perform an action
            2 => {
                let sleep_time = unsafe { rand_int_range(self.gen, 0, 25) };
                thread::sleep(Duration::from_millis(sleep_time as u64));
                let action_or_no = unsafe { rand_int_range(self.gen, 0, 5) };
                if action_or_no > 3 {
                    self.get_action(t)
                } else {
                    None
                }
            },
            // do nothing at all in response to the tick
            _ => None,
        };

        action_opt
    }
}

// Make sure that the values we pull out of the seeded random number generator really are deterministic.
#[test]
fn do_test() {
    unsafe {
        let gen1 = init_rng(12345u32);
        let gen2 = init_rng(12345u32);

        let rand1 = rand_int_range(gen1, 1i32, 1000000i32);
        let rand2 = rand_int_range(gen2, 1i32, 1000000i32);

        assert_eq!(rand1, rand2);
    }
}

pub struct EventLogger {
    i: usize, // incremented every time an event is logged in order to record order
}

impl EventLogger {
    pub fn new() -> EventLogger {
        EventLogger {
            i: 0,
        }
    }
    /// Logs an event taking place during the fuzzing process.
    pub fn log_request(&mut self, action: &BrokerAction) {
        self.i += 1;
        unimplemented!();
    }

    pub fn log_response(&mut self, res: &BrokerResult) {
        self.i += 1;
        unimplemented!();
    }
}
