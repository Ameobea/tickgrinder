//! Simulated broker used for backtests.  Contains facilities for simulating trades,
//! managing balances, and reporting on statistics from previous trades.
//!
//! See README.md for more information about the specifics of the SimBroker implementation
//! and a description of its functionality.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::collections::BinaryHeap;
use std::sync::mpsc;
use std::thread;
use std::ops::{Index, IndexMut};
use std::mem;
#[allow(unused_imports)]
use test;

use futures::{Future, Sink, oneshot, Oneshot, Complete};
use futures::stream::Stream;
use futures::sync::mpsc::{unbounded, channel, UnboundedReceiver, UnboundedSender, Sender, Receiver};
use uuid::Uuid;

use tickgrinder_util::trading::tick::*;
pub use tickgrinder_util::trading::broker::*;
use tickgrinder_util::trading::trading_condition::*;
use tickgrinder_util::transport::command_server::CommandServer;

mod tests;
mod helpers;
pub use self::helpers::*;
mod client;
pub use self::client::*;

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
    push_stream_handle: Option<Sender<BrokerResult>>,
    /// A handle to the receiver for the channel throgh which push messages are received
    push_stream_recv: Option<Receiver<BrokerResult>>,
    /// The CommandServer used for logging
    pub cs: CommandServer,
}

// .-.
unsafe impl Send for SimBroker {}

impl SimBroker {
    pub fn new(
        settings: SimBrokerSettings, cs: CommandServer, client_rx: UnboundedReceiver<(BrokerAction, Complete<BrokerResult>)>
    ) -> SimBroker {
        let mut accounts = Accounts::new();
        // create with one account with the starting balance.
        let account = Account {
            uuid: Uuid::new_v4(),
            ledger: Ledger::new(settings.starting_balance),
            live: false,
        };
        accounts.insert(Uuid::new_v4(), account);
        // TODO: Make sure that 0 is the right buffer size for this channel
        let (client_push_tx, client_push_rx) = channel::<BrokerResult>(0);
        let (mpsc_tx, mpsc_rx) = mpsc::sync_channel(0);

        // spawn a thread to block on the `client_rx` and map it into the mpsc so we can conditionally check for new values.
        // Eventually, we'll want to use a threadsafe binary heap to avoid this behind-the-scenes involved with this.
        thread::spawn(move || {
            for msg in client_rx.wait() {
                mpsc_tx.send(msg.unwrap()).unwrap();
            }
        });

        SimBroker {
            accounts: accounts,
            settings: settings,
            symbols: Symbols::new(cs.clone()),
            pq: SimulationQueue::new(),
            timestamp: 0,
            client_rx: Some(mpsc_rx),
            push_stream_handle: Some(client_push_tx),
            push_stream_recv: Some(client_push_rx),
            cs: cs,
        }
    }

    /// Starts the simulation process.  Ticks are read in from the inputs and processed internally into
    /// priority queue.  The source of blocking here is determined by the client.  The `Stream`s of `Tick`s
    /// that are handed off have a capacity of 1 so that the `Sender` will block until it is consumed by
    /// the client.
    ///
    /// The assumption is that the client will do all its processing and have a chance to
    /// submit `BrokerActions` to the SimBroker before more processing more ticks, thus preserving the
    /// strict ordering by timestamp of events and fully simulating asynchronous operation.
    pub fn init_sim_loop(mut self) {
        // initialize the internal queue with values from attached tickstreams
        // all tickstreams should be added by this point
        self.pq.init(&mut self.symbols);
        self.cs.debug(None, "Internal simulation queue has been initialized.");

        // continue looping while the priority queue has new events to simulate
        while let Some(item) = self.pq.pop() {
            self.timestamp = item.timestamp;
            // first check if we have any messages from the client to process into the queue
            while let Ok((action, complete,)) = self.client_rx.as_mut().unwrap().try_recv() {
                // determine how long it takes the broker to process this message internally
                let execution_delay = self.settings.get_delay(&action);
                // insert this message into the internal queue adding on processing time
                let qi = QueueItem {
                    timestamp: self.timestamp + execution_delay,
                    unit: WorkUnit::ActionComplete(complete, action),
                };
                self.pq.push(qi);
            }

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
                    self.tick_positions(symbol_ix, (tick.bid, tick.ask,));
                    // push the next future tick into the queue
                    self.pq.push_next_tick(&mut self.symbols);
                },
                // A tick arriving at the client.  We now send it down the Client's channels and block
                // until it is consumed.
                WorkUnit::ClientTick(symbol_ix, tick) => {
                    // TODO: Check to see if this does a copy and if it does, fine a way to eliminate it
                    let mut inner_symbol = &mut self.symbols[symbol_ix];
                    if tick.timestamp % 100000 == 0 {
                        println!("{}", tick.timestamp);
                    }
                    // send the tick through the client stream, blocking until it is consumed by the client.
                    inner_symbol.send_client(tick);
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
                    future.complete(res.clone());
                    // send the push message through the channel, blocking until it's consumed by the client.
                    self.push_msg(res);
                },
            }
        }

        // if we reach here, that means we've run out of ticks to simulate from the input streams.
        let ts_string = self.timestamp.to_string();
        self.cs.notice(
            Some(&ts_string),
            "All input tickstreams have ran out of ticks; internal simulation loop stopped."
        );
    }

    /// Immediately sends a message over the broker's push channel.  Should only be called from within
    /// the SimBroker's internal event handling loop since it immediately sends the message.
    fn push_msg(&mut self, msg: BrokerResult) {
        let sender = mem::replace(&mut self.push_stream_handle, None).unwrap();
        let new_sender = sender.send(msg).wait().expect("Unable to push_msg");
        mem::replace(&mut self.push_stream_handle, Some(new_sender));
    }

    /// Returns a handle with which to send push messages.  The returned handle will immediately send
    /// messages to the client so should only be used from within the internal event handling loop.
    fn get_push_handle(&self) -> Sender<Result<BrokerMessage, BrokerError>> {
        self.push_stream_handle.clone().unwrap()
    }

    /// Initializes the push stream by creating internal messengers
    fn init_stream() -> (mpsc::Sender<Result<BrokerMessage, BrokerError>>, UnboundedReceiver<BrokerResult>) {
        let (mpsc_s, mpsc_r) = mpsc::channel::<Result<BrokerMessage, BrokerError>>();
        let tup = unbounded::<BrokerResult>();
        // wrap the sender in options so we can `mem::replace` them in the loop.
        let (mut f_s, f_r) = (Some(tup.0), tup.1);

        thread::spawn(move || {
            // block until message received over a mpsc sender
            // then re-transmit them through the push stream
            for message in mpsc_r.iter() {
                match message {
                    Ok(message) => {
                        let temp_f_s = mem::replace(&mut f_s, None).unwrap();
                        let new_f_s = temp_f_s.send(Ok(message)).wait().unwrap();
                        mem::replace(&mut f_s, Some(new_f_s));
                    },
                    Err(err_msg) => {
                        let temp_f_s = mem::replace(&mut f_s, None).unwrap();
                        let new_f_s = temp_f_s.send(Err(err_msg)).wait().unwrap();
                        mem::replace(&mut f_s, Some(new_f_s));
                    },
                }
            }
            println!("After init_stream() channel conversion loop!!");
        });

        (mpsc_s, f_r)
    }

    /// Actually carries out the action of the supplied BrokerAction (simulates it being received and processed)
    /// by a remote broker) and returns the result of the action.  The provided timestamp is that of
    /// when it was received by the broker (after delays and simulated lag).
    fn exec_action(&mut self, cmd: &BrokerAction) -> BrokerResult {
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
            &BrokerAction::Disconnect => unimplemented!(),
        }
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

        // check if we're able to open this position right away at market price
        match order.is_open_satisfied(bid, ask) {
            // if this order is fillable right now, open it.
            Some(entry_price) => {
                let res = self.market_open(account_uuid, symbol_ix, long, size, stop, take_profit, Some(0));
                // this should always succeed
                assert!(res.is_ok());
                return res
            },
            None => (),
        }

        let pos_value = self.get_position_value(&order)?;

        // if we're not able to open it, try to place the order.
        let res = match self.accounts.entry(account_uuid) {
            Entry::Occupied(mut o) => {
                let account = o.get_mut();
                let margin_requirement = pos_value / self.settings.leverage;
                account.ledger.place_order(order.clone(), margin_requirement)
            },
            Entry::Vacant(_) => {
                Err(BrokerError::NoSuchAccount)
            },
        };

        // if the order was actually placed, notify the cache that we've opened a new order
        match &res {
            &Ok(ref msg) => {
                match msg {
                    &BrokerMessage::OrderPlaced{order_id, order: _, timestamp: _} => {
                        self.accounts.order_placed(&order, order_id, account_uuid)
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
        &mut self, account_id: Uuid, symbol_ix: usize, long: bool, size: usize, stop: Option<usize>,
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

        let pos_value = self.get_position_value(&pos)?;
        let pos_uuid = Uuid::new_v4();

        let res;
        { // borrow-b-gone
            let account_ = self.accounts.entry(account_id);
            res = match account_ {
                Entry::Occupied(mut occ) => {
                    let mut account = occ.get_mut();
                    // subtract the cost of the position from our balance
                    if account.ledger.balance < pos_value * self.settings.leverage {
                        return Err(BrokerError::InsufficientBuyingPower);
                    } else {
                        account.ledger.balance -= pos_value * self.settings.leverage;
                    }

                    // create the position in the `Ledger`
                    account.ledger.open_position(pos_uuid, pos.clone())
                },
                Entry::Vacant(_) => {
                    return Err(BrokerError::NoSuchAccount);
                }
            };
        }

        // that should never fail
        assert!(res.is_ok());
        // add the position to the cache for checking when to close it
        self.accounts.position_opened(&pos, pos_uuid);

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

        let pos;
        { // borrow-b-gone
            let account = match self.accounts.entry(account_id) {
                Entry::Occupied(o) => o.into_mut(),
                Entry::Vacant(_) => {
                    return Err(BrokerError::NoSuchAccount);
                },
            };

            pos = match account.ledger.open_positions.entry(position_uuid) {
                Entry::Occupied(o) => o.get().clone(),
                Entry::Vacant(_) => {
                    return Err(BrokerError::NoSuchPosition);
                }
            };
        }

        let pos_value = self.get_position_value(&pos)?;
        let res = { // borrow-b-gone
            let account = self.accounts.get_mut(&account_id).unwrap();

            let modification_cost = (pos_value / pos.size) * size;
            account.ledger.resize_position(position_uuid, (-1 * size as isize), modification_cost, self.timestamp)
        };

        // if the position was fully closed, remove it from the cache
        match res {
            Ok(ref message) => match message {
                &BrokerMessage::PositionClosed{position: ref pos, position_id: pos_uuid, reason: _, timestamp: _} => {
                    self.accounts.position_closed(pos, pos_uuid);
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
                    assert!(res.is_ok());
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
        let res = {
            let account = match self.accounts.entry(account_uuid) {
                Entry::Occupied(o) => o.into_mut(),
                Entry::Vacant(_) => {
                    return Err(BrokerError::NoSuchAccount);
                },
            };

            // attempt to cancel the order and remove it from the hashmaps
            account.ledger.cancel_order(order_uuid, self.timestamp)
        };

        // if it was successful, remove the position from the `pending` cache.
        match res {
            Ok(ref msg) => {
                match msg {
                    &BrokerMessage::OrderCancelled{ ref order, order_id: _, timestamp: _ } => {
                        self.accounts.order_cancelled(order_uuid, order.symbol_id)
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

    /// Returns the worth of a position in units of base currency.
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
    /// if they need to be opened, closed, or modified in any way due to this update.  All actions that take place here are
    /// guarenteed to succeed since they are simulated as taking place within the brokerage itself.  All `BrokerMessage`s
    /// generated by any actions that take place are sent through the supplied push stream handle to the client.
    pub fn tick_positions(&mut self, symbol_id: usize, price: (usize, usize)) {
        let (bid, ask) = price;
        // check if any pending orders should be closed, modified, or opened
        // manually keep track of the index because we remove things from the vector dynamically
        let mut i = 0;
        while i < self.accounts.positions[symbol_id].pending.len() {
            let push_msg_opt;
            { // borrow-b-gone
                let &CachedPosition { pos_uuid, acct_uuid, ref pos } = &self.accounts.positions[symbol_id].pending[i];
                push_msg_opt = match pos.is_open_satisfied(bid, ask) {
                    Some(open_price) => {
                        // if the position should be opened, remove it from the pending `HashMap` and the cache and open it.
                        let mut ledger = &mut self.accounts.data.get_mut(&acct_uuid).unwrap().ledger;
                        // remove from the hashmap
                        let mut hm_pos = ledger.pending_positions.remove(&pos_uuid).unwrap();
                        hm_pos.execution_price = Some(open_price);
                        hm_pos.execution_time = Some(self.timestamp);

                        Some(ledger.open_position(pos_uuid, pos.clone()))
                    },
                    None => None,
                };
            }

            if push_msg_opt.is_some() {
                // remove from the pending cache
                let swapped_pos = self.accounts.positions[symbol_id].pending.remove(i);
                // add it to the open cache
                self.accounts.positions[symbol_id].open.push(swapped_pos);
                let push_msg = push_msg_opt.unwrap();
                // this should always succeed
                assert!(push_msg.is_ok());
                // send the push message to the client
                self.push_msg(Ok(push_msg.unwrap()));
                // decrement i since we modified the cache
                i -= 1;
            }

            i += 1;
        }

        // check if any open positions should be closed or modified
        let mut i = 0;
        while i < self.accounts.positions[symbol_id].open.len() {
            let push_msg_opt;
            { // borrow-b-gone
                let &CachedPosition { pos_uuid, acct_uuid, ref pos } = &self.accounts.positions[symbol_id].open[i];
                push_msg_opt = match pos.is_close_satisfied(bid, ask) {
                    Some((closure_price, closure_reason)) => {
                        let pos_value = self.get_position_value(&pos).expect("Unable to get position value for pending position!");
                        // if the position should be closed, remove it from the cache.
                        let mut ledger = &mut self.accounts.data.get_mut(&acct_uuid).unwrap().ledger;
                        // remove from the hashmap
                        let mut real_pos = ledger.open_positions.remove(&pos_uuid).unwrap();
                        real_pos.exit_price = Some(closure_price);
                        real_pos.exit_time = Some(self.timestamp);

                        Some(ledger.close_position(pos_uuid, pos_value, self.timestamp, closure_reason))
                    },
                    None => None,
                };
            }

            if push_msg_opt.is_some() {
                // remove from the open cache
                let _ = self.accounts.positions[symbol_id].open.remove(i);
                let push_msg = push_msg_opt.unwrap();
                // this should always succeed
                assert!(push_msg.is_ok());
                // send the push message to the client
                self.push_msg(push_msg);
                // decrement i since we modified the cache
                i -= 1;
            }

            i += 1;
        }
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
        &mut self, name: String, raw_tickstream: UnboundedReceiver<Tick>, is_fx: bool, decimal_precision: usize
    ) -> BrokerResult {
        // allocate space for open positions of the new symbol in `Accounts`
        self.accounts.add_symbol();
        let mut sym = Symbol::new_from_stream(raw_tickstream.boxed(), is_fx, decimal_precision, name.clone());
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

/// Only enable event-level debug information to be logged if we have need to.
#[cfg(feature = "superlog")]
pub fn event_log(timestamp: u64, event: &str) {
    unimplemented!();
}

#[cfg(not(feature = "superlog"))]
#[allow(unused_variables)]
pub fn event_log(timestamp: u64, event: &str) {
    // Do nothing if we're not looking for event-level debugging.
    // this should optimize out completely, leaving zero overhead.
}
