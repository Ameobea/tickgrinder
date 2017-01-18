//! Broker fuzzer.  See README.md for a full description.

use std::collections::HashMap;
use libc::c_void;
use rand::{self, Rng};

use futures::{Future, Complete};

use tickgrinder_util::strategies::Strategy;
use tickgrinder_util::trading::broker::Broker;
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
}

impl Strategy for Fuzzer {
    fn new(cs: CommandServer, qs: QueryServer) -> Fuzzer {
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

        Fuzzer {
            gen: unsafe { init_rng(seed)},
            logger: EventLogger::new(),
        }
    }

    fn init(&mut self) {
        // first step is to initialize the connection to the broker
        let broker = ActiveBroker::init(HashMap::new()).wait().unwrap().unwrap();
    }

    fn exit_now(&mut self, ready: Complete<()>) {
        unimplemented!();
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
    pub fn event_log(&mut self, msg: &str) {
        self.i += 1;
        unimplemented!();
    }
}
