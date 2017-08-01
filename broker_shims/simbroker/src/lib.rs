//! Simulated broker used for backtests.  Contains facilities for simulating trades,
//! managing balances, and reporting on statistics from previous trades.
//!
//! See README.md for more information about the specifics of the SimBroker implementation
//! and a description of its functionality.

#![feature(libc, rustc_attrs, core_intrinsics, conservative_impl_trait, associated_consts, custom_derive, test, slice_patterns, rand)]

extern crate test;
extern crate futures;
extern crate uuid;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate tickgrinder_util;
#[macro_use]
extern crate from_hashmap;
extern crate libc;
extern crate rand;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::collections::BinaryHeap;
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::ops::{Index, IndexMut};
use std::mem;
use libc::c_void;

use futures::{Stream, oneshot, Oneshot, Complete};
use futures::stream::BoxStream;
use futures::sync::mpsc::{channel, Sender};
use uuid::Uuid;
use rand::Rng;

use tickgrinder_util::trading::tick::*;
pub use tickgrinder_util::trading::broker::*;
use tickgrinder_util::trading::trading_condition::*;
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::tickstream::{TickGenerator, TickGenerators};
use tickgrinder_util::conf::CONF;

mod tests;
mod helpers;
pub use self::helpers::*;
mod client;
pub use self::client::*;
mod superlog;
use superlog::SuperLogger;

// link with the libboost_random wrapper
#[link(name="rand_bindings")]
extern {
    fn init_rng(seed: u32) -> *mut c_void;
    fn rand_int_range(void_rng: *mut c_void, min: i32, max: i32) -> u32;
}

/// A simulated broker that is used as the endpoint for trading activity in backtests.  This is the broker backend
/// that creates/ingests streams that interact with the client.
pub struct SimBroker {
    /// Contains all the accounts simulated by the SimBroker
    pub accounts: Accounts,
    /// A copy of the settings generated from the input HashMap
    pub settings: SimBrokerSettings,
    /// Contains the streams that yield `Tick`s for the SimBroker as well as data about the symbols and other metadata.
    symbols: Symbols,
    /// Priority queue that maintains that forms the basis of the internal ordered event loop.
    pq: SimulationQueue,
    /// Timestamp of last price update received by broker
    timestamp: u64,
    /// Receiving end of the channel over which the `SimBrokerClient` sends messages
    client_rx: Option<mpsc::Receiver<(BrokerAction, Complete<BrokerResult>)>>,
    /// A handle to the sender for the channel through which push messages are sent
    push_stream_handle: Option<Sender<(u64, BrokerResult)>>,
    /// A handle to the receiver for the channel through which push messages are received
    push_stream_recv: Option<Box<Stream<Item=(u64, BrokerResult), Error=()> + Send>>,
    /// The CommandServer used for logging
    pub cs: CommandServer,
    /// Holds a logger used to log detailed data to flatfile if the `superlog` feature id enabled and an empty struct otherwise.
    logger: SuperLogger,
    /// A source of deterministic PRNG to be used to generating Uuids.
    prng: *mut c_void,
}

// .-.
unsafe impl Send for SimBroker {}

impl SimBroker {
    pub fn new(
        settings: SimBrokerSettings, cs: CommandServer, client_rx: mpsc::Receiver<(BrokerAction, Complete<BrokerResult>)>,
    ) -> Result<SimBroker, BrokerError> {
        let logger = SuperLogger::new();
        let mut accounts = Accounts::new(logger.clone());

        // set up the deterministicly random data generator if it's enabled in the config
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
        let rng = unsafe { init_rng(seed) };
        let uuid = gen_uuid(rng);

        // create with one account with the starting balance.
        let account = Account {
            uuid: uuid,
            ledger: Ledger::new(settings.starting_balance),
            live: false,
        };
        accounts.insert(uuid, account);
        // TODO: Make sure that 0 is the right buffer size for this channel
        let (client_push_tx, client_push_rx) = channel::<(u64, BrokerResult)>(0);

        // try to deserialize the "tickstreams" parameter of the input settings to get a list of tickstreams register
        let tickstreams: Vec<(String, TickGenerators, bool, usize)> = serde_json::from_str(&settings.tickstreams)
            .map_err(|_| BrokerError::Message{message: String::from("Unable to deserialize the input tickstreams into a vector!")})?;

        let mut sim = SimBroker {
            accounts: accounts,
            settings: settings,
            symbols: Symbols::new(cs.clone()),
            pq: SimulationQueue::new(),
            timestamp: 0,
            client_rx: Some(client_rx),
            push_stream_handle: Some(client_push_tx),
            push_stream_recv: Some(client_push_rx.boxed()),
            cs: cs,
            logger: logger,
            prng: rng,
        };

        // create an actual tickstream for each of the definitions and subscribe to all of them
        for (name, def, is_fx, decimals) in tickstreams {
            let mut gen: Box<TickGenerator> = def.get();
            let strm = gen.get_raw().map_err(|s| BrokerError::Message{message: s})?;
            sim.register_tickstream(name, strm, is_fx, decimals)?;
        }

        Ok(sim)
    }

    /// Starts the simulation process.  Ticks are read in from the inputs and processed internally into
    /// priority queue.  The source of blocking here is determined by the client.  The `Stream`s of `Tick`s
    /// that are handed off have a capacity of 1 so that the `Sender` will block until it is consumed by
    /// the client.
    ///
    /// The assumption is that the client will do all its processing and have a chance to
    /// submit `BrokerActions` to the SimBroker before more processing more ticks, thus preserving the
    /// strict ordering by timestamp of events and fully simulating asynchronous operation.
    pub fn init_sim_loop(&mut self) {
        // initialize the internal queue with values from attached tickstreams
        // all tickstreams should be added by this point
        self.pq.init(&mut self.symbols);
        self.cs.debug(None, "Internal simulation queue has been initialized.");
        self.logger.event_log(self.timestamp, "Starting the great simulation loop...");
    }

    /// Called by the fuzzer executor to drive progress on the simulation.  Returns the number of client
    /// actions (tickstream ticks + pushstream messages) that were sent to the client during this tick.
    pub fn tick_sim_loop(&mut self, num_last_actions: usize, buffer: &mut Vec<TickOutput>) -> usize {
        // first check if we have any messages from the client to process into the queue
        { // borrow-b-gone
            let rx = self.client_rx.as_mut().unwrap();
            for _ in 0..num_last_actions {
                // get the next message from the client receiver
                // println!("Blocking for message from client...");
                let (action, complete) = rx.recv().expect("Error from client receiver!");
                // println!("Got message from client: {:?}", action);
                // determine how long it takes the broker to process this message internally
                let execution_delay = self.settings.get_delay(&action);
                // insert this message into the internal queue adding on processing time
                let qi = QueueItem {
                    timestamp: self.timestamp + execution_delay,
                    unit: WorkUnit::ActionComplete(complete, action),
                };
                self.logger.event_log(self.timestamp, &format!("Pushing new ActionComplete into pq: {:?}", qi.unit));
                self.pq.push(qi);
            }
        }

        if self.timestamp % 100000 == 0 {
            self.cs.notice(None, &format!("{} ticks processed", self.timestamp));
        }

        let item = self.pq.pop().unwrap();
        self.timestamp = item.timestamp;
        let mut client_event_count = 0;

        // then process the new item we took out of the queue
        match item.unit {
            // A tick arriving at the broker.  The client doesn't get to know until after network delay.
            WorkUnit::NewTick(symbol_ix, tick) => {
                // update the price for the popped tick's symbol
                let price = (tick.bid, tick.ask);
                self.symbols[symbol_ix].price = price;
                // push the ClientTick event back into the queue + network delay
                self.pq.push(QueueItem {
                    timestamp: tick.timestamp as u64 + self.settings.ping_ns,
                    unit: WorkUnit::ClientTick(symbol_ix, tick),
                });
                // check to see if we have any actions to take on open positions and take them if we do
                self.logger.event_log(
                    self.timestamp,
                    &format!("Ticking positions in response to new tick: ({}, {:?})", symbol_ix, tick)
                );
                client_event_count += self.tick_positions(symbol_ix, (tick.bid, tick.ask,), client_event_count, buffer);
                // push the next future tick into the queue
                self.logger.event_log(self.timestamp, &format!("Pushing ClientTick into queue: ({}, {:?})", symbol_ix, tick));
                self.pq.push_next_tick(&mut self.symbols);
            },
            // A tick arriving at the client.  We now send it down the Client's channels and block
            // until it is consumed.
            WorkUnit::ClientTick(symbol_ix, tick) => {
                // TODO: Check to see if this does a copy and if it does, fine a way to eliminate it
                let mut inner_symbol = &mut self.symbols[symbol_ix];
                self.logger.event_log(self.timestamp, &format!("Sending tick to client: ({}, {:?})", symbol_ix, tick));
                // send the tick through the client stream, blocking until it is consumed by the client.
                inner_symbol.send_client(tick);
                // put the message into the result buffer and increment its length
                buffer[client_event_count] = TickOutput::Tick(symbol_ix, tick);
                client_event_count += 1;
            },
            // The moment the broker finishes processing an action and the action takes place.
            // Begins the network delay for the trip back to the client.
            WorkUnit::ActionComplete(future, action) => {
                // process the message and re-insert the response into the queue
                assert_eq!(self.timestamp, item.timestamp);
                let res = self.exec_action(&action);
                // calculate when the response would be recieved by the client
                // then re-insert the response into the queue
                let res_time = item.timestamp + self.settings.ping_ns;
                let item = QueueItem {
                    timestamp: res_time,
                    unit: WorkUnit::Response(future, res),
                };
                self.pq.push(item);
            },
            // The moment a response reaches the client.
            WorkUnit::Response(future, res) => {
                // fulfill the future with the result
                match res {
                    Ok(BrokerMessage::AccountListing{accounts: _}) => {
                        let msg = "Fulfilling work unit Ok(AccountListing{_})'s oneshot";
                        self.logger.event_log(self.timestamp, msg);
                    },
                    Ok(BrokerMessage::Ledger{ledger: _}) => {
                        let msg = "Fulfilling work unit Ok(Ledger{_})'s oneshot";
                        self.logger.event_log(self.timestamp, msg);
                    },
                    _ => self.logger.event_log(self.timestamp, &format!("Fulfilling work unit {:?}'s oneshot", res)),
                };
                future.complete(res.clone());
                // send the push message through the channel, blocking until it's consumed by the client.
                self.push_msg(res.clone());
                // put the message into the result buffer and increment its length
                buffer[client_event_count] = TickOutput::Pushstream(self.timestamp, res);
                client_event_count += 1;
            },
            // The moment a spurious notification reaches the client.  Network delay is already taken intou account,
            // so we can deliver it immediately.
            WorkUnit::Notification(res) => {
                self.logger.event_log(self.timestamp, &format!("Delivering spurious notification to client: {:?}", res));
                // send the push message through the channel, blocking until it's consumed by the client.
                self.push_msg(res.clone());
                // put the message into the result buffer and increment its length
                buffer[client_event_count] = TickOutput::Pushstream(self.timestamp, res);
                client_event_count += 1;
            }
        }

        client_event_count
    }

    /// Immediately sends a message over the broker's push channel.  Should only be called from within
    /// the SimBroker's internal event handling loop since it immediately sends the message.
    fn push_msg(&mut self, _: BrokerResult) {
        // self.logger.event_log(self.timestamp, &format!("`push_msg()` sending message to client: {:?}", msg));
        // let sender = mem::replace(&mut self.push_stream_handle, None).unwrap();
        // let new_sender = sender.send((self.timestamp, msg)).wait().expect("Unable to push_msg");
        // mem::replace(&mut self.push_stream_handle, Some(new_sender));
    }

    /// Actually carries out the action of the supplied BrokerAction (simulates it being received and processed)
    /// by a remote broker) and returns the result of the action.  The provided timestamp is that of
    /// when it was received by the broker (after delays and simulated lag).
    fn exec_action(&mut self, cmd: &BrokerAction) -> BrokerResult {
        self.logger.event_log(self.timestamp, &format!("`exec_action()`: {:?}", cmd));
        match cmd {
            &BrokerAction::Ping => {
                Ok(BrokerMessage::Pong{time_received: self.timestamp})
            },
            &BrokerAction::TradingAction{account_uuid, ref action} => {
                match action {
                    &TradingAction::MarketOrder{ref symbol, long, size, stop, take_profit, max_range} => {
                        match self.symbols.get_index(symbol) {
                            Some(ix) => self.market_open(account_uuid, ix, long, size, stop, take_profit, max_range),
                            None => Err(BrokerError::NoSuchSymbol),
                        }
                    },
                    &TradingAction::MarketClose{uuid, size} => {
                        self.market_close(account_uuid, uuid, size)
                    },
                    &TradingAction::LimitOrder{ref symbol, long, size, stop, take_profit, entry_price} => {
                        match self.symbols.get_index(symbol) {
                            Some(ix) => self.place_order(account_uuid, ix, entry_price, long, size, stop, take_profit),
                            None => Err(BrokerError::NoSuchSymbol),
                        }
                    },
                    // no support for partial closes at this time
                    &TradingAction::LimitClose{uuid, size, exit_price} => {
                        // limit close just means to take profit when we hit a certain price, so just adjust the TP
                        self.modify_position(account_uuid, uuid, None, Some(Some(exit_price)))
                    },
                    &TradingAction::ModifyOrder{uuid, size, entry_price, stop, take_profit} => {
                        self.modify_order(account_uuid, uuid, size, entry_price, stop, take_profit)
                    },
                    &TradingAction::CancelOrder{uuid} => {
                        self.cancel_order(account_uuid, uuid)
                    }
                    &TradingAction::ModifyPosition{uuid, stop, take_profit} => {
                        self.modify_position(account_uuid, uuid, Some(stop), Some(take_profit))
                    },
                }
            },
            &BrokerAction::GetLedger{account_uuid} => {
                match self.accounts.get(&account_uuid) {
                    Some(acct) => Ok(BrokerMessage::Ledger{ledger: acct.ledger.clone()}),
                    None => Err(BrokerError::NoSuchAccount),
                }
            },
            &BrokerAction::ListAccounts => {
                let mut res = Vec::with_capacity(self.accounts.len());
                for (_, acct) in self.accounts.iter() {
                    res.push(acct.clone());
                }
                Ok(BrokerMessage::AccountListing{accounts: res})
            }
            &BrokerAction::Disconnect => unimplemented!(),
        }
    }

    /// Called when the balance of a ledger has been changed.  Automatically takes into account ping.
    fn buying_power_changed(&mut self, account_uuid: Uuid, new_buying_power: usize) {
        self.pq.push(QueueItem{
            timestamp: self.timestamp + self.settings.ping_ns,
            unit: WorkUnit::Notification(Ok(BrokerMessage::LedgerBalanceChange{
                account_uuid: account_uuid,
                new_buying_power: new_buying_power,
            })),
        });
    }

    /// Creates a new pending position on the `SimBroker`.
    fn place_order(
        &mut self, account_uuid: Uuid, symbol_ix: usize, limit_price: usize, long: bool, size: usize,
        stop: Option<usize>, take_profit: Option<usize>,

    ) -> BrokerResult {
        let opt = self.get_price(symbol_ix);
        if opt.is_none() {
            return Err(BrokerError::NoSuchSymbol)
        }
        let (bid, ask) = opt.unwrap();

        let order = Position {
            creation_time: self.timestamp,
            symbol_id: symbol_ix,
            size: size,
            price: Some(limit_price),
            long: long,
            stop: stop,
            take_profit: take_profit,
            execution_time: None,
            execution_price: None,
            exit_price: None,
            exit_time: None,
        };

        // make sure the supplied parameters are sane
        let _ = order.check_sanity()?;

        // check if we're able to open this position right away at market price
        match order.is_open_satisfied(bid, ask) {
            // if this order is fillable right now, open it.
            Some(entry_price) => {
                let res = self.market_open(account_uuid, symbol_ix, long, size, stop, take_profit, Some(0));
                // this should always succeed
                if res.is_err() {
                    self.logger.error_log(&format!("Error while trying to place order: {:?}, {:?}", &order, res));
                }
                // assert!(res.is_ok());
                return res
            },
            None => (),
        }

        let pos_value = self.get_position_value(&order)?;

        // if we're not able to open it, try to place the order.
        let res = match self.accounts.entry(account_uuid) {
            Entry::Occupied(mut o) => {
                let account = o.get_mut();
                account.ledger.place_order(order.clone(), pos_value, gen_uuid(self.prng))
            },
            Entry::Vacant(_) => {
                Err(BrokerError::NoSuchAccount)
            },
        };

        // if the order was actually placed, notify the cache that we've opened a new order
        // also send notification of ledger buying power change
        match &res {
            &Ok(ref msg) => {
                match msg {
                    &BrokerMessage::OrderPlaced{order_id, order: _, timestamp: _} => {
                        self.accounts.order_placed(&order, order_id, account_uuid);
                        let new_buying_power = self.accounts.get(&account_uuid).unwrap().ledger.buying_power;
                        self.buying_power_changed(account_uuid, new_buying_power);
                    },
                    _ => (),
                }
            },
            &Err(_) => (),
        }

        res
    }

    /// Attempts to open a position at the current market price with options for settings stop loss, or take profit.
    /// Right now, this assumes that the order is filled as soon as it is placed (after the processing delay is taken
    /// into account) and that it is filled fully.
    fn market_open(
        &mut self, account_uuid: Uuid, symbol_ix: usize, long: bool, size: usize, stop: Option<usize>,
        take_profit: Option<usize>, max_range: Option<usize>
    ) -> BrokerResult {
        let opt = self.get_price(symbol_ix);
        if opt.is_none() {
            return Err(BrokerError::NoSuchSymbol)
        }
        let (bid, ask) = opt.unwrap();

        let cur_price = if long { ask } else { bid };

        let pos = Position {
            creation_time: self.timestamp,
            symbol_id: symbol_ix,
            size: size,
            price: Some(cur_price),
            long: long,
            stop: stop,
            take_profit: take_profit,
            execution_time: Some(self.timestamp + self.settings.execution_delay_ns),
            execution_price: Some(cur_price),
            exit_price: None,
            exit_time: None,
        };

        // make sure the supplied parameters are sane
        let _ = pos.check_sanity()?;

        let pos_value = self.get_position_value(&pos)?;
        let pos_uuid = gen_uuid(self.prng);

        let new_buying_power;
        let res = {
            let acct_entry = self.accounts.entry(account_uuid);
            match acct_entry {
                Entry::Occupied(mut occ) => {
                    let mut account = occ.get_mut();
                    // manually subtract the cost of the position from the account balance
                    if account.ledger.buying_power < pos_value {
                        return Err(BrokerError::InsufficientBuyingPower);
                    } else {
                        account.ledger.buying_power -= pos_value;
                        new_buying_power = account.ledger.buying_power;
                    }

                    // create the position in the `Ledger`
                    account.ledger.open_position(pos_uuid, pos.clone())
                },
                Entry::Vacant(_) => {
                    return Err(BrokerError::NoSuchAccount);
                }
            }
        };

        // that should never fail
        assert!(res.is_ok());
        // add the position to the cache for checking when to close it
        self.accounts.position_opened_immediate(&pos, pos_uuid, account_uuid);
        // send notification about the change in ledger buying power
        self.buying_power_changed(account_uuid, new_buying_power);

        res
    }

    /// Attempts to close part of a position at market price.  Right now, this assumes that the order is
    /// fully filled as soon as it is placed (after the processing delay is taken into account).
    fn market_close(&mut self, account_id: Uuid, position_uuid: Uuid, size: usize) -> BrokerResult {
        if size == 0 {
            let ts_string = self.timestamp.to_string();
            self.cs.warning(
                Some(&ts_string),
                &format!("Warning: Attempted to close 0 units of position with uuid {}", position_uuid)
            );
            // TODO: Add configuration setting to optionally return an error
        }

        let pos = {
            let account = match self.accounts.entry(account_id) {
                Entry::Occupied(o) => o.into_mut(),
                Entry::Vacant(_) => {
                    return Err(BrokerError::NoSuchAccount);
                },
            };

            match account.ledger.open_positions.entry(position_uuid) {
                Entry::Occupied(o) => o.get().clone(),
                Entry::Vacant(_) => {
                    return Err(BrokerError::NoSuchPosition);
                }
            }
        };

        let pos_value = self.get_position_value(&pos)?;

        let new_buying_power;
        let res = {
            let account = self.accounts.get_mut(&account_id).unwrap();
            let modification_cost = (pos_value / pos.size) * size;
            let res = account.ledger.resize_position(position_uuid, (-1 * size as isize), modification_cost, self.timestamp);
            new_buying_power = account.ledger.buying_power;
            res
        };

        // if the position was fully closed, remove it from the cache and send notification of ledger buying power change
        match res {
            Ok(ref message) => match message {
                &BrokerMessage::PositionClosed{position: ref pos, position_id: pos_uuid, reason: _, timestamp: _} => {
                    self.accounts.position_closed(pos, pos_uuid);
                    self.buying_power_changed(account_id, new_buying_power);
                },
                _ => (),
            },
            Err(_) => (),
        }
        res
    }

    /// Modifies an order, setting the parameters of the contained `Position` equal to those supplied.
    fn modify_order(
        &mut self, account_uuid: Uuid, pos_uuid: Uuid, size: usize, entry_price: usize,
        stop: Option<usize>, take_profit: Option<usize>,
    ) -> BrokerResult {
        let res = {
            let order = {
                let account = match self.accounts.entry(account_uuid) {
                    Entry::Occupied(o) => o.into_mut(),
                    Entry::Vacant(_) => {
                        return Err(BrokerError::NoSuchAccount);
                    },
                };

                // pull it out of the pending hashmap while we modify it
                match account.ledger.pending_positions.get(&pos_uuid) {
                    Some(pos) => pos,
                    None => {
                        return Err(BrokerError::NoSuchPosition);
                    },
                }.clone()
            };
            let opt = self.get_price(order.symbol_id);
            if opt.is_none() {
                return Err(BrokerError::NoSuchSymbol)
            }
            let (bid, ask) = opt.unwrap();
            match order.is_open_satisfied(bid, ask) {
                // if the new entry price makes the order marketable, go ahead and open the position.
                Some(entry_price) => {
                    let res = {
                        let account = self.accounts.get_mut(&account_uuid).unwrap();
                        // remove the position from the pending hashmap
                        let mut hm_order = account.ledger.pending_positions.remove(&pos_uuid).unwrap();
                        hm_order.execution_time = Some(self.timestamp);
                        hm_order.execution_price = Some(entry_price);
                        // add it to the open hashmap
                        account.ledger.open_position(pos_uuid, order.clone())
                    };
                    // that should always succeed
                    if res.is_err() {
                        self.logger.error_log(&format!("Error while trying to modify order: {:?}, {:?}", &order, res));
                    }
                    // assert!(res.is_ok());
                    // notify the cache that the position was opened
                    self.accounts.position_opened(&order, pos_uuid);
                    return res;
                },
                // if it's not marketable, perform the modification on the ledger
                None => {
                    let mut account = self.accounts.get_mut(&account_uuid).unwrap();
                    account.ledger.modify_order(pos_uuid, size, entry_price, stop, take_profit, self.timestamp)
                },
            }
        };

        // as of now, the modification operation always succeeds so we should always update the cache
        match res.as_ref().unwrap() {
            &BrokerMessage::OrderModified{ ref order, order_id: _, timestamp: _ } => {
                self.accounts.order_modified(order, pos_uuid);
            },
            _ => unreachable!(),
        }

        res
    }

    /// Cancels the pending position.
    pub fn cancel_order(&mut self, account_uuid: Uuid, order_uuid: Uuid) -> BrokerResult {
        let new_buying_power;
        let res = {
            let account = match self.accounts.entry(account_uuid) {
                Entry::Occupied(o) => o.into_mut(),
                Entry::Vacant(_) => {
                    return Err(BrokerError::NoSuchAccount);
                },
            };
            // attempt to cancel the order and remove it from the hashmaps
            let res = account.ledger.cancel_order(order_uuid, self.timestamp);
            new_buying_power = account.ledger.buying_power;
            res
        };

        // if it was successful, remove the position from the `pending` cache
        // also send notification of ledger buying power change
        match res {
            Ok(ref msg) => {
                match msg {
                    &BrokerMessage::OrderCancelled{ ref order, order_id: _, timestamp: _ } => {
                        self.accounts.order_cancelled(order_uuid, order.symbol_id);
                        self.buying_power_changed(account_uuid, new_buying_power);
                    },
                    _ => unreachable!(),
                }
            },
            Err(_) => (),
        }

        res
    }

    /// Modifies the stop loss or take profit of a position.  SL and TP are double option-wrapped; the outer
    /// option indicates if they should be changed and the inner option indicates if the value should be set
    /// or not (`Some(None)` indicates that the current SL should be removed, for example).
    fn modify_position(
        &mut self, account_id: Uuid, position_uuid: Uuid, sl: Option<Option<usize>>, tp: Option<Option<usize>>
    ) -> BrokerResult {
        let res = {
            let account = match self.accounts.entry(account_id) {
                Entry::Occupied(o) => o.into_mut(),
                Entry::Vacant(_) => {
                    return Err(BrokerError::NoSuchAccount);
                },
            };
            account.ledger.modify_position(position_uuid, sl, tp, self.timestamp)
        };

        // TODO: Check if the new SL/TP make the position meet closure conditions and if they do, close it

        // if the position was actually modified, remove it from the cache
        match res {
            Ok(ref message) => match message {
                &BrokerMessage::PositionModified{position: ref pos, position_id: pos_uuid, timestamp: _} => {
                    self.accounts.position_modified(pos, pos_uuid);
                },
                _ => (),
            },
            Err(_) => (),
        }
        res
    }

    /// Dumps the SimBroker state to a file that can be resumed later.
    fn dump_to_file(&mut self, filename: &str) {
        unimplemented!(); // TODO
    }

    /// Used for Forex exchange rate conversions.  The cost to open a position is determined
    /// by the exchange rate between the base currency and the primary currency of the pair.
    /// A decimal precision of 10 is used for all returned results.
    ///
    /// Gets the conversion rate (in pips) between the base currency of the simbroker and
    /// the supplied currency.  If the base currency is USD and AUD is provided, the exchange
    /// rate for AUD/USD will be returned.  Returns Err if we lack the data to do that.  Results
    /// are returned with the specified decimal precision.
    fn get_base_rate(&self, currency: &str, desired_decimals: usize) -> Result<usize, BrokerError> {
        if !self.settings.fx {
            return Err(BrokerError::Message{
                message: String::from("Can only convert to base rate when in FX mode.")
            });
        }

        let base_currency = &self.settings.fx_base_currency;
        let base_pair = format!("{}{}", currency, base_currency);

        let (_, ask, decimals) = if !self.symbols.contains(&base_pair) {
            // try reversing the order or the pairs
            let base_pair_reverse = format!("{}{}", base_currency, currency);
            if !self.symbols.contains(&base_pair_reverse) {
                return Err(BrokerError::NoDataAvailable);
            } else {
                self.symbols[&base_pair_reverse].get_price()
            }
        } else {
            self.symbols[&base_pair].get_price()
        };

        Ok(convert_decimals(ask, decimals, desired_decimals))
    }

    /// Returns the value of a position in units of base currency, not taking into account leverage.
    fn get_position_value(&self, pos: &Position) -> Result<usize, BrokerError> {
        let ix = pos.symbol_id;

        let sym = &self.symbols[ix];
        if sym.is_fx() {
            let base_rate: usize = self.get_base_rate(&sym.name[0..3], sym.metadata.decimal_precision)?;
            Ok(pos.size * base_rate * self.settings.fx_lot_size)
        } else {
            Ok(pos.size)
        }
    }

    /// Called every price update the broker receives.  It simulates some kind of market activity on the simulated exchange
    /// that triggers a price update for that symbol.  This function checks all pending and open positions and determines
    /// if they need to be opened, closed, or modified in any way due to this update.
    ///
    /// All actions that take place here are guarenteed to succeed since they are simulated as taking place within the
    /// brokerage itself.  All `BrokerMessage`s generated by any actions that take place are sent through the supplied
    /// push stream handle to the client.  The returned value is how many push messages were sent to the client
    /// during this tick.
    pub fn tick_positions(
        &mut self, symbol_id: usize, price: (usize, usize), cur_index: usize, buffer: &mut Vec<TickOutput>
    ) -> usize {
        let (bid, ask) = price;
        let mut push_msg_count = 0;
        // check if any pending orders should be closed, modified, or opened
        // manually keep track of the index because we remove things from the vector dynamically
        let mut i = 0;
        while i < self.accounts.positions[symbol_id].pending.len() {
            let push_msg_opt = {
                let &CachedPosition { pos_uuid, acct_uuid, ref pos } = &self.accounts.positions[symbol_id].pending[i];
                match pos.is_open_satisfied(bid, ask) {
                    Some(open_price) => {
                        // if the position should be opened, remove it from the pending `HashMap` and the cache and open it.
                        let mut ledger = &mut self.accounts.data.get_mut(&acct_uuid).unwrap().ledger;
                        // remove from the hashmap
                        let mut hm_pos = ledger.pending_positions.remove(&pos_uuid).unwrap();
                        hm_pos.execution_price = Some(open_price);
                        hm_pos.execution_time = Some(self.timestamp);

                        Some(ledger.open_position(pos_uuid, hm_pos))
                    },
                    None => None,
                }
            };

            i += 1;

            match push_msg_opt {
                Some(Ok(BrokerMessage::PositionOpened{position_id: _, position: ref hm_pos, timestamp: _})) => {
                    // remove from the pending cache
                    let mut cached_pos = self.accounts.positions[symbol_id].pending.remove(i-1);
                    // update the cached position with the one with execution data
                    cached_pos.pos = hm_pos.clone();
                    let push_msg = push_msg_opt.as_ref().unwrap();
                    // this should always succeed
                    // if push_msg.is_err() {
                    //     let err_msg = format!("Error while trying to open position during tick check: {:?}, {:?}", &cached_pos.pos, push_msg);
                    //     self.logger.error_log(&err_msg);
                    // }
                    assert!(push_msg.is_ok());
                    // add it to the open cache
                    self.accounts.positions[symbol_id].open.push(cached_pos);
                    // send the push message to the client
                    self.push_msg(Ok(push_msg.as_ref().unwrap().clone()));
                    // put the new tick into the buffer to be returned to the client
                    let output = TickOutput::Pushstream(self.timestamp, Ok(push_msg.as_ref().unwrap().clone()));
                    buffer[cur_index + push_msg_count] = output;
                    push_msg_count += 1;
                    // decrement i since we modified the cache
                    i -= 1;
                },
                Some(Err(err)) => self.logger.error_log(&format!("Push message from opening pending position was error: {:?}", err)),
                Some(Ok(msg)) => self.logger.error_log(&format!("Received unexpected response type when opening pending position: {:?}", msg)),
                None => (),
            }
        }

        // check if any open positions should be closed or modified
        let mut i = 0;
        while i < self.accounts.positions[symbol_id].open.len() {
            let mut new_buying_power = 0;
            let push_msg_opt: Option<(usize, BrokerResult)> = {
                let &CachedPosition { pos_uuid, acct_uuid, ref pos } = &self.accounts.positions[symbol_id].open[i];
                match pos.is_close_satisfied(bid, ask) {
                    Some((closure_price, closure_reason)) => {
                        let pos_value = self.get_position_value(&pos).expect("Unable to get position value for pending position!");
                        // if the position should be closed, remove it from the cache.
                        let mut ledger = &mut self.accounts.data.get_mut(&acct_uuid).unwrap().ledger;

                        let res = ledger.close_position(pos_uuid, pos_value, self.timestamp, closure_reason);
                        new_buying_power = ledger.buying_power;
                        Some((closure_price, res))
                    },
                    None => None,
                }
            };

            i += 1;

            if push_msg_opt.is_some() {
                let (closure_price, push_msg) = push_msg_opt.unwrap();
                // remove from the open cache
                let mut cached_pos = self.accounts.positions[symbol_id].open.remove(i-1);
                cached_pos.pos.exit_price = Some(closure_price);
                cached_pos.pos.exit_time = Some(self.timestamp);
                // this should always succeed
                assert!(push_msg.is_ok());
                // send notification of ledger buying power change to client
                let buying_power_notification = BrokerMessage::LedgerBalanceChange{
                    account_uuid: cached_pos.acct_uuid,
                    new_buying_power: new_buying_power,
                };
                let output = TickOutput::Pushstream(self.timestamp, Ok(buying_power_notification));
                // add the message to the buffer and increment the length
                buffer[cur_index + push_msg_count] = output;
                push_msg_count += 1;
                // send the push message to the client
                self.push_msg(push_msg.clone());
                // put the new tick into the buffer to be returned to the client
                let output = TickOutput::Pushstream(self.timestamp, push_msg);
                // add the message to the buffer and increment the length
                buffer[cur_index + push_msg_count] = output;
                push_msg_count += 1;
                // decrement i since we modified the cache
                i -= 1;
            }
        }

        push_msg_count
    }

    /// Sets the price for a symbol.  If no Symbol currently exists with that designation, a new one
    /// will be initialized with a static price.
    fn oneshot_price_set(
        &mut self, name: String, price: (usize, usize), is_fx: bool, decimal_precision: usize,
    ) {
        if is_fx {
            assert_eq!(name.len(), 6);
        }

        // insert new entry into `self.prices` or update if one exists
        if self.symbols.contains(&name) {
            self.symbols[&name].price = price;
        } else {
            let symbol = Symbol::new_oneshot(price, is_fx, decimal_precision, name.clone());
            self.symbols.add(name, symbol).expect("Unable to set oneshot price for new symbol");
        }
    }

    /// Returns a clone of an account's ledger or an error if it doesn't exist.
    pub fn get_ledger_clone(&mut self, account_uuid: Uuid) -> Result<Ledger, BrokerError> {
        match self.accounts.get(&account_uuid) {
            Some(acct) => Ok(acct.ledger.clone()),
            None => Err(BrokerError::Message{
                message: "No account exists with that UUID.".to_string()
            }),
        }
    }

    /// Registers a data source into the SimBroker.  Ticks from the supplied generator will be
    /// used to upate the SimBroker's internal prices and transmitted to connected clients.
    pub fn register_tickstream(
        &mut self, name: String, raw_tickstream: BoxStream<Tick, ()>, is_fx: bool, decimal_precision: usize
    ) -> BrokerResult {
        // allocate space for open positions of the new symbol in `Accounts`
        self.accounts.add_symbol();
        let mut sym = Symbol::new_from_stream(raw_tickstream, is_fx, decimal_precision, name.clone());
        // get the first element out of the tickstream and set the next tick equal to it
        let first_tick = sym.next().unwrap().unwrap();
        self.cs.debug(None, &format!("Set first tick for tickstream {}: {:?}", name, &first_tick));
        sym.next_tick = Some(first_tick);
        self.symbols.add(name, sym)
    }

    /// Returns the current price for a given symbol or None if the SimBroker
    /// doensn't have a price.
    pub fn get_price(&self, ix: usize) -> Option<(usize, usize)> {
        if !self.symbols.len() > ix {
            return Some(self.symbols[ix].price)
        }

        None
    }
}
