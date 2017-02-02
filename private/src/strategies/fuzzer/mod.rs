//! Broker fuzzer.  See README.md for a full description.

use std::collections::HashMap;
use std::thread;
use std::sync::mpsc;

use libc::c_void;
use rand::{self, Rng};

use futures::{Future, Stream, Sink, Canceled};
use futures::stream::BoxStream;
use futures::sync::mpsc::{channel, Sender};
use futures::sync::oneshot;
use uuid::Uuid;

use tickgrinder_util::strategies::{ManagedStrategy, Helper, StrategyAction, Tickstream, Merged};
use tickgrinder_util::trading::broker::BrokerResult;
use tickgrinder_util::trading::objects::{BrokerAction, BrokerMessage, Account, Ledger};
use tickgrinder_util::trading::tick::{Tick, GenTick};
use tickgrinder_util::trading::trading_condition::TradingAction;
use tickgrinder_util::transport::textlog::get_logger_handle;
use tickgrinder_util::conf::CONF;

// link with the libboost_random wrapper
#[link(name="rand_bindings")]
extern {
    fn init_rng(seed: u32) -> *mut c_void;
    fn rand_int_range(void_rng: *mut c_void, min: i32, max: i32) -> u32;
}

fn random_bool(void_rng: *mut c_void) -> bool {
    if unsafe { rand_int_range(void_rng, 0, 1) } == 1 { true } else { false }
}

pub struct FuzzerState {
    account_uuid: Option<Uuid>,
    account: Option<Account>,
}

impl FuzzerState {
    pub fn new() -> FuzzerState {
        FuzzerState {
            account_uuid: None,
            account: None,
        }
    }
}

impl FuzzerState {
    pub fn get_ledger(&mut self) -> &mut Ledger {
        &mut self.account.as_mut().unwrap().ledger
    }
}

pub struct Fuzzer {
    pub gen: *mut c_void,
    pub logger: EventLogger,
    pub events_tx: Option<Sender<oneshot::Receiver<BrokerResult>>>,
    pub events_rx: mpsc::Receiver<BrokerResult>,
    pub state: FuzzerState,
}

impl Fuzzer {
    pub fn new(conf: HashMap<String, String>) -> Fuzzer {
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

        // create the stream over which we receive callbacks from the broker
        let buffer_size = 2048; // let's hope that's big enough.
        let (tx, rx) = channel(0);

        // map the output of the events buffer stream into a stdlib mpsc channel so we can try_get it
        let (mpsc_tx, mpsc_rx) = mpsc::sync_channel::<BrokerResult>(0);
        thread::spawn(move || {
            let mod_rx: BoxStream<oneshot::Receiver<BrokerResult>, Canceled> = rx.map_err(|()| Canceled).boxed();
            let buf_rx = mod_rx.buffer_unordered(buffer_size);
            for msg in buf_rx.wait() {
                mpsc_tx.send(msg.unwrap()).unwrap();
            }
        });

        Fuzzer {
            gen: unsafe { init_rng(seed)},
            logger: EventLogger::new(),
            events_tx: Some(tx),
            events_rx: mpsc_rx,
            state: FuzzerState::new(),
        }
    }

    pub fn get_logger(&self) -> EventLogger {
        self.logger.clone()
    }
}

impl ManagedStrategy<()> for Fuzzer {
    #[allow(unused_variables)]
    fn init(&mut self, helper: &mut Helper, subscriptions: &[Tickstream]) {
        let mut logger = self.logger.clone();
        logger.log_misc(String::from("`init()` called"));
        let accounts = helper.broker.list_accounts().wait().expect("Unable to get accounts.").unwrap();
        for (uuid, account) in accounts.iter() {
            self.state.account_uuid = Some(*uuid);
            self.state.account = Some(account.clone());
        }
    }

    fn tick(&mut self, helper: &mut Helper, gt: &GenTick<Merged<()>>) -> Option<StrategyAction> {
        while let Ok(msg) = self.events_rx.try_recv() {
            self.logger.log_misc(format!("EVENT: {:?}", msg));
        }

        let (data_ix, ref t) = match gt.data {
            Merged::BrokerTick(ix, t) => (ix, t),
            Merged::BrokerPushstream(ref res) => {
                self.logger.log_pushtream(gt.timestamp, res);
                handle_pushstream(&mut self.state, res, self.gen);
                return None;
            },
            Merged::T(_) => panic!("Got custom type but we don't have one."),
        };

        self.logger.log_tick(t, data_ix);
        let action = get_action(&mut self.state, t, self.gen);
        match action {
            Some(ref strategy_action) => {
                match strategy_action {
                    &StrategyAction::BrokerAction(ref broker_action) => {
                        self.logger.log_request(broker_action, t.timestamp);
                    },
                    _ => unimplemented!(),
                }
            },
            None => (),
        };

        action
    }

    fn abort(&mut self) {
        unimplemented!();
    }
}

/// Called during each iteration of the fuzzer loop.  Picks a random action to take based on the
/// internally held PRNG and executes it.
pub fn get_action(state: &mut FuzzerState, t: &Tick, rng: *mut c_void) -> Option<StrategyAction> {
    let rand = unsafe { rand_int_range(rng, 0, 5) };
    match rand {
        0 => Some(StrategyAction::BrokerAction(BrokerAction::Ping)),
        1 => { // random market open order
            let price = unsafe { rand_int_range(rng, 25, 75) } as usize;
            let order = TradingAction::MarketOrder{
                symbol: String::from("TEST"),
                long: random_bool(rng),
                size: unsafe { rand_int_range(rng, 0, 5) as usize },
                stop: if random_bool(rng) { Some(price + unsafe { rand_int_range(rng, 0, 5) as usize }) } else { None },
                max_range: None,
                take_profit: if random_bool(rng) { Some(price + unsafe { rand_int_range(rng, 0, 5) as usize }) } else { None },
            };
            Some(StrategyAction::BrokerAction(BrokerAction::TradingAction{
                account_uuid: state.account_uuid.unwrap(),
                action: order
            }))
        },
        2 => { // add one more level of chaos to this beautifully deterministic system
            let action_or_no = unsafe { rand_int_range(rng, 0, 5) };
            if action_or_no > 3 {
                get_action(state, t, rng)
            } else {
                None
            }
        },
        // do nothing at all in response to the tick
        _ => None,
    }
}

/// Process a pushstream message
pub fn handle_pushstream(state: &mut FuzzerState, msg: &BrokerResult, rng: *mut c_void) {
    match msg {
        &Ok(ref evt) => {
            match evt {
                // update our internal view of the account whenever an order/position is opened/modified
                // TODO: Store cancelled orders in SimBroker
                &BrokerMessage::OrderPlaced{order_id, ref order, timestamp: _} => {
                    state.get_ledger().pending_positions.insert(order_id, order.clone()).unwrap();
                },
                &BrokerMessage::OrderModified{order_id, ref order, timestamp: _} => {
                    let ledger = state.get_ledger();
                    assert!(ledger.pending_positions.get(&order_id).is_some());
                    ledger.pending_positions.insert(order_id, order.clone()).unwrap();
                },
                &BrokerMessage::OrderCancelled{order_id, ref order, timestamp: _} => {
                    let cancelled_order = state.get_ledger().pending_positions.remove(&order_id).unwrap();
                    assert_eq!(&cancelled_order, order);
                }
                &BrokerMessage::PositionOpened{ref position_id, ref position, timestamp: _} => {
                    let ledger = state.get_ledger();
                    ledger.pending_positions.remove(position_id).unwrap();
                    ledger.open_positions.insert(*position_id, position.clone()).unwrap();
                },
                &BrokerMessage::PositionModified{position_id, ref position, timestamp: _} => {
                    let ledger = state.get_ledger();
                    assert!(ledger.open_positions.get(&position_id).is_some());
                    ledger.open_positions.insert(position_id, position.clone()).unwrap();
                },
                &BrokerMessage::PositionClosed{position_id, ref position, reason: _, timestamp: _} => {
                    let ledger = state.get_ledger();
                    ledger.open_positions.remove(&position_id).unwrap();
                    ledger.closed_positions.insert(position_id, position.clone()).unwrap();
                },
                _ => (),
            }
        },
        &Err(_) => (),
    }
}

#[derive(Clone)]
pub struct EventLogger {
    tx: Option<Sender<String>>,
}

impl EventLogger {
    /// Initializes a new logger thread and returns handle to it
    /// TODO: write header info into the log file about symbol/symbol_id pairing etc.
    pub fn new() -> EventLogger {
        let tx = get_logger_handle(String::from("fuzzer"), 50);

        EventLogger {
            tx: Some(tx),
        }
    }

    /// Logs an event taking place during the fuzzing process.  Returns a number to be used to match
    /// the request to a response.
    pub fn log_request(&mut self, action: &BrokerAction, timestamp: u64) {
        // println!("Sending request to broker: {:?}", action);
        let tx = self.tx.take().unwrap();
        let new_tx = tx.send(format!("{} - REQUEST: {:?}", timestamp, action))
            .wait().expect("Unable to log request!");
        self.tx = Some(new_tx);
    }

    pub fn log_pushtream(&mut self, timestamp: u64, res: &BrokerResult) {
        // println!("Got pushstream message: {:?}", res);
        let tx = self.tx.take().unwrap();
        let new_tx = tx.send(format!("{} - PUSHSTREAM: {:?}", timestamp, res))
            .wait().expect("Unable to log pushtream message!");
        self.tx = Some(new_tx);
    }

    /// Logs a response received from the broker
    pub fn log_response(&mut self, res: &BrokerResult, id: usize) {
        // println!("Got response from broker: {:?}", res);
        let tx = self.tx.take().unwrap();
        let new_tx = tx.send(format!("{} - RESPONSE: {:?}", id, res))
            .wait().expect("Unable to log response!");
        self.tx = Some(new_tx);
    }

    /// Logs the fuzzer receiving a tick from the broker.  `i` is the index of that symbol.
    pub fn log_tick(&mut self, t: &Tick, i: usize) {
        // println!("Received new tick from broker: {:?}", t);
        let tx = self.tx.take().unwrap();
        let new_tx = tx.send(format!("Received tick from symbol with index {}: {:?}", i, t))
            .wait().expect("Unable to log tick!");
        self.tx = Some(new_tx);
    }

    /// Logs a plain old text message
    pub fn log_misc(&mut self, msg: String) {
        // println!("Message: {}", msg);
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
