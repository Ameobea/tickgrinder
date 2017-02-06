//! Broker fuzzer.  See README.md for a full description.

use std::collections::HashMap;

use libc::c_void;
use rand::{self, Rng};

use futures::{Future, Sink};
use futures::sync::mpsc::Sender;
use uuid::Uuid;

use tickgrinder_util::strategies::{ManagedStrategy, Helper, StrategyAction, Tickstream, Merged};
use tickgrinder_util::trading::broker::{Broker, BrokerResult};
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
    pub state: FuzzerState,
}

impl Fuzzer {
    pub fn new(_: HashMap<String, String>) -> Fuzzer {
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

        Fuzzer {
            gen: unsafe { init_rng(seed)},
            logger: EventLogger::new(),
            state: FuzzerState::new(),
        }
    }

    pub fn get_logger(&self) -> EventLogger {
        self.logger.clone()
    }
}

impl<B: Broker> ManagedStrategy<B, ()> for Fuzzer {
    #[allow(unused_variables)]
    fn init(&mut self, helper: &mut Helper<B>, subscriptions: &[Tickstream]) {
        let mut logger = self.logger.clone();
        logger.log_misc(String::from("`init()` called"));
        let res = helper.broker.execute(BrokerAction::ListAccounts).wait().expect("Unable to get accounts.");
        match res {
            Ok(BrokerMessage::AccountListing{accounts}) => {
                for account in accounts.iter() {
                    self.state.account_uuid = Some(account.uuid);
                    self.state.account = Some(account.clone());
                }
            },
            Ok(_) => panic!("Received weird response type from ListAccounts!"),
            Err(_) => panic!("ListAccounts returned an error!"),
        }
    }

    fn tick(&mut self, _: &mut Helper<B>, gt: &GenTick<Merged<()>>) -> Option<StrategyAction> {
        let (data_ix, ref t) = match gt.data {
            Merged::BrokerTick(ix, t) => (ix, t),
            Merged::BrokerPushstream(ref res) => {
                self.logger.log_pushtream(gt.timestamp, res);
                handle_pushstream(&mut self.state, res, self.gen);
                return None;
            },
            Merged::T(_) => panic!("Got custom type but we don't have one defined."),
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
    let rand = unsafe { rand_int_range(rng, 0, 20) };
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
        2 => { // add one more level of chaos to this beautifully yet deterministic system
            let action_or_no = unsafe { rand_int_range(rng, 0, 5) };
            if action_or_no > 3 {
                get_action(state, t, rng)
            } else {
                None
            }
        },
        3 => { // random limit order
            let price = unsafe { rand_int_range(rng, 25, 75) } as usize;
            let order = TradingAction::LimitOrder{
                symbol: String::from("TEST"),
                long: random_bool(rng),
                size: unsafe { rand_int_range(rng, 0, 5) as usize },
                stop: if random_bool(rng) { Some(price + unsafe { rand_int_range(rng, 0, 5) as usize }) } else { None },
                take_profit: if random_bool(rng) { Some(price + unsafe { rand_int_range(rng, 0, 5) as usize }) } else { None },
                entry_price: price,
            };

            Some(StrategyAction::BrokerAction(BrokerAction::TradingAction{
                account_uuid: state.account_uuid.unwrap(),
                action: order,
            }))
        },
        8 ... 11 => { // maybe close part of or all of an open position at market
            let account_uuid = state.account_uuid.unwrap();
            let ledger = state.get_ledger();
            let open_pos_count = ledger.open_positions.len();
            // the more positions open, the higher the chance that we close one.
            let roll = unsafe { rand_int_range(rng, 0, (open_pos_count + 1) as i32) } as usize;
            if roll >= open_pos_count {
                return None;
            }

            let mut i = 0;
            for (uuid, pos) in ledger.pending_positions.iter() {
                if i == roll {
                    return Some(StrategyAction::BrokerAction(BrokerAction::TradingAction{
                        account_uuid: account_uuid,
                        action: TradingAction::MarketClose{
                            uuid: *uuid,
                            size: unsafe { rand_int_range(rng, 0, pos.size as i32 + 2) } as usize,
                        }
                    }));
                }

                i += 1;
            }

            None // do nothing if we have no open positions
        },
        12 ... 15 => { // maybe close an open position with a limit close
            let account_uuid = state.account_uuid.unwrap();
            let ledger = state.get_ledger();
            let open_pos_count = ledger.open_positions.len();
            // the more positions open, the higher the chance that we close one.
            let roll = unsafe { rand_int_range(rng, 0, (open_pos_count + 5) as i32) } as usize;
            if roll >= open_pos_count {
                return None;
            }

            let roll = unsafe { rand_int_range(rng, 0, 15) } as i32;
            let mut i = 0;
            for (uuid, pos) in ledger.pending_positions.iter() {
                if i == roll {
                    return Some(StrategyAction::BrokerAction(BrokerAction::TradingAction{
                        account_uuid: account_uuid,
                        action: TradingAction::LimitClose{
                            uuid: *uuid,
                            size: unsafe { rand_int_range(rng, 0, pos.size as i32 + 1) } as usize,
                            exit_price: if t.bid as i32 >= roll && random_bool(rng) {
                                (t.bid as i32 - roll) as usize
                            } else {
                                (t.bid as i32 + roll) as usize
                            },
                        }
                    }));
                }

                i += 1;
            }

            None // do nothing if we have no open positions
        }
        16 => { // cancel a pending order
            // only go forward half the time
            if random_bool(rng) {
                return None;
            }

            // TODO: This may be a source of indeterminism; check into that.
            //       It almost certainly will be until the deterministic Uuid generation is implemented.
            let account_uuid = state.account_uuid.unwrap();
            for (uuid, _) in state.get_ledger().pending_positions.iter() {
                // cancel the first order returned by the iterator
                return Some(StrategyAction::BrokerAction(BrokerAction::TradingAction{
                    account_uuid: account_uuid,
                    action: TradingAction::CancelOrder{uuid: *uuid},
                }))
            }

            None // there were no pending orders to cancel
        },
        17 => { // Get a copy of the ledger and make sure it's the same as the one we have
            Some(StrategyAction::BrokerAction(BrokerAction::GetLedger{account_uuid: state.account_uuid.unwrap()}))
        },
        18 => { // Request an account listing
            Some(StrategyAction::BrokerAction(BrokerAction::ListAccounts))
        }
        // do nothing at all in response to the tick
        _ => None,
    }
}

/// Process a pushstream message
pub fn handle_pushstream(state: &mut FuzzerState, msg: &BrokerResult, _: *mut c_void) {
    match msg {
        &Ok(ref evt) => {
            match evt {
                // update our internal view of the account whenever an order/position is opened/modified
                // TODO: Store cancelled orders in SimBroker
                &BrokerMessage::OrderPlaced{order_id, ref order, timestamp: _} => {
                    state.get_ledger().pending_positions.insert(order_id, order.clone());
                },
                &BrokerMessage::OrderModified{order_id, ref order, timestamp: _} => {
                    let ledger = state.get_ledger();
                    assert!(ledger.pending_positions.get(&order_id).is_some());
                    ledger.pending_positions.insert(order_id, order.clone());
                },
                &BrokerMessage::OrderCancelled{order_id, ref order, timestamp: _} => {
                    let cancelled_order = state.get_ledger().pending_positions.remove(&order_id).unwrap();
                    assert_eq!(&cancelled_order, order);
                }
                &BrokerMessage::PositionOpened{ref position_id, ref position, timestamp: _} => {
                    let ledger = state.get_ledger();
                    let _ = ledger.pending_positions.remove(position_id);
                    ledger.open_positions.insert(*position_id, position.clone());
                },
                &BrokerMessage::PositionModified{position_id, ref position, timestamp: _} => {
                    let ledger = state.get_ledger();
                    assert!(ledger.open_positions.get(&position_id).is_some());
                    ledger.open_positions.insert(position_id, position.clone());
                },
                &BrokerMessage::PositionClosed{position_id, ref position, reason: _, timestamp: _} => {
                    let ledger = state.get_ledger();
                    ledger.open_positions.remove(&position_id).unwrap();
                    ledger.closed_positions.insert(position_id, position.clone());
                },
                &BrokerMessage::Ledger{ref ledger} => {
                    assert_eq!(*ledger, state.account.as_ref().unwrap().ledger);
                },
                &BrokerMessage::LedgerBalanceChange{account_uuid, new_buying_power} => {
                    if account_uuid == state.account_uuid.unwrap() {
                        let mut ledger = &mut state.account.as_mut().unwrap().ledger;
                        ledger.buying_power = new_buying_power;
                    }
                }
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
        let msg = match res {
            &Ok(BrokerMessage::AccountListing{accounts: _}) => format!("{} - PUSHSTREAM: Ok(AccountListing{{_}}", timestamp),
            &Ok(BrokerMessage::Ledger{ledger: _}) => format!("{} - PUSHSTREAM: Ok(Ledger{{_}}", timestamp),
            _ => format!("{} - PUSHSTREAM: {:?}", timestamp, res),
        };
        // println!("Got pushstream message: {:?}", res);
        let tx = self.tx.take().unwrap();
        let new_tx = tx.send(msg)
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
