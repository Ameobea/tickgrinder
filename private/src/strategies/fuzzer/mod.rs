//! Broker fuzzer.  See README.md for a full description.

use std::collections::HashMap;
use std::thread;
use std::path::PathBuf;
use std::fs::{DirBuilder, ReadDir, read_dir, File};
use std::time::Duration;
use std::io::Write;

use libc::c_void;
use rand::{self, Rng};
use time::now;

use futures::{Future, Stream, Complete};
use futures::sync::mpsc::{unbounded, channel, UnboundedSender};

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
        1 => unimplemented!(), // TODO
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
    tx: UnboundedSender<String>,
    i: usize, // incremented every time an event is logged in order to record order
}

impl EventLogger {
    /// Initializes a new logger thread and returns handle to it
    /// TODO: write header info into the log file about symbol/symbol_id pairing etc.
    pub fn new() -> EventLogger {
        let (tx, rx) = unbounded();

        // spawn the logger thread and initialize the logging loop
        thread::spawn(move || {
            // if the directories don't exist in the logging directory, create them
            let log_dir: PathBuf = PathBuf::from(CONF.data_dir).join("logs").join("fuzzer");
            if !log_dir.is_dir() {
                let mut builder = DirBuilder::new();
                builder.recursive(true).create(log_dir.clone())
                    .expect("Unable to create directory to hold the log files; permission issue or bad data dir configured?");
            }

            let mut attempts = 1;
            let curtime = now();
            let datestring = format!("{}-{}_{}.log", curtime.tm_mon + 1, curtime.tm_mday, attempts);
            while PathBuf::from(CONF.data_dir).join("logs").join("fuzzer").join(&datestring).exists() {
                attempts += 1;
            }

            let datestring = format!("{}-{}_{}.log", curtime.tm_mon + 1, curtime.tm_mday, attempts);
            let mut file = File::create(PathBuf::from(CONF.data_dir).join("logs").join("fuzzer").join(&datestring))
                .expect("Unable to create log file!");

            // buffer up 50 log lines before writing to disk
            for msg in rx.chunks(50).wait() {
                let text: String = match msg {
                    Ok(lines) => lines.as_slice().join("\n") + "\n",
                    // World is likely dropping due to a crash or shutdown
                    Err(_) => unimplemented!(),
                };

                // write the 50 lines into the file
                write!(&mut file, "{}", text);
            }
        });

        EventLogger {
            tx: tx,
            i: 0,
        }
    }

    /// Logs an event taking place during the fuzzing process.  Returns a number to be used to match
    /// the request to a response.
    pub fn log_request(&mut self, action: &BrokerAction) -> usize {
        self.i += 1;
        self.tx.send(format!("{} -  REQUEST: {:?}", self.i, action))
            .expect("Unable to log request!");
        self.i
    }

    /// Logs a response received from the broker
    pub fn log_response(&mut self, res: &BrokerResult, id: usize) {
        self.tx.send(format!("{} - RESPONSE: {:?}", id, res)).expect("Unable to log response!");
    }

    /// Logs the fuzzer receiving a tick from the broker.  `i` is the index of that symbol.
    pub fn log_tick(&mut self, t: Tick, i: usize) {
        self.tx.send(format!("Received tick from symbol with index {}: {:?}", i, t))
            .expect("Unable to log tick!");
    }

    /// Logs a plain old text message
    pub fn log_misc(&mut self, msg: String) {
        self.tx.send(msg).expect("Logging tick failed");
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
