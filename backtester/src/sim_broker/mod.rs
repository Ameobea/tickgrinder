//! Simulated broker used for backtests.  Contains facilities for simulating trades,
//! managing balances, and reporting on statistics from previous trades.
//!
//! See README.md for more information about the specifics of the SimBroker implementation
//! and a description of its functionality.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::collections::BinaryHeap;
use std::sync::atomic::{Ordering, AtomicUsize};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::ops::{Index, IndexMut};
use std::mem;
#[allow(unused_imports)]
use test;

use futures::{Future, Sink, oneshot, Oneshot, Complete};
use futures::stream::{BoxStream, Stream};
use futures::sync::mpsc::{unbounded, channel, UnboundedReceiver, UnboundedSender, Sender, Receiver};
use uuid::Uuid;

use tickgrinder_util::trading::tick::*;
pub use tickgrinder_util::trading::broker::*;
use tickgrinder_util::trading::trading_condition::*;
use tickgrinder_util::transport::command_server::CommandServer;

/// Contains metadata about a particular tickstream and the symbol of the ticks
/// that it holds
pub struct SymbolData {
    /// `true` if the ticks are an exchange rate
    /// The symbol must be six characters like "EURUSD"
    pub is_fx: bool,
    /// Decimal precision of the input ticks
    pub decimal_precision: usize,
}

/// Represents a BrokerAction submitted by a client that's waiting to be processed by
/// the SimBroker due to simulated network latency or some other simulated delay.
struct PendingAction {
    pub future: Complete<BrokerResult>,
    pub action: BrokerAction,
}

impl PartialEq for PendingAction {
    fn eq(&self, other: &PendingAction) -> bool {
        self.action == other.action
    }
}

impl Eq for PendingAction {}

/// A unit of execution or the internal timestamp-ordered event loop.
enum WorkUnit {
    /// Simulates trading events triggering a new Tick for a particular symbol.
    /// Allocating Strings for each tick would be way too expensive, so indexes of
    /// managed ticks are used instead.
    Tick(usize, Tick),
    /// Simulates an action being received from a client.
    PendingAction(Complete<BrokerResult>, BrokerAction),
    /// Simulates a message from the broker being received by a client.
    Response(Complete<BrokerResult>, BrokerResult),
}

impl PartialEq for WorkUnit {
    fn eq(&self, other: &WorkUnit) -> bool {
        match *self {
            WorkUnit::Tick(self_ix, self_tick) => {
                match *other {
                    WorkUnit::Tick(other_ix, other_tick) => {
                        self_ix == other_ix && self_tick == other_tick
                    },
                    _ => false,
                }
            },
            WorkUnit::PendingAction(_, ref self_action) => {
                match *other {
                    WorkUnit::PendingAction(_, ref other_action) => {
                        self_action == other_action
                    },
                    _ => false,
                }
            },
            WorkUnit::Response(_, ref self_res) => {
                match *other {
                    WorkUnit::Response(_, ref other_res) => {
                        self_res == other_res
                    },
                    _ => false,
                }
            }
        }
    }
}

impl Eq for WorkUnit {}

/// A timestamped unit of data for the priority queue.
#[derive(PartialEq, Eq)]
struct QueueItem {
    timestamp: u64,
    unit: WorkUnit,
}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &QueueItem) -> Option<::std::cmp::Ordering> {
        Some(self.timestamp.cmp(&other.timestamp))
    }
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

pub struct Symbol {
    /// The input stream that yields the ticks.  This is consumed internally and its `Tick`s
    /// consumed into the priority queue.
    pub input_stream: Option<BoxStream<Tick, ()>>,
    /// the stream that is mapped into that which is handed off to the client.  Only yields
    /// `Tick`s when the order of events dictates it inside the internal simulation loop.
    pub client_stream: Option<Sender<Tick>>,
    /// Contains some information about the symbol that the ticks represent
    pub metadata: SymbolData,
    /// Broker's view of prices in pips, determined by the `tick_receiver`s
    pub price: Arc<(AtomicUsize, AtomicUsize)>,
}

impl Symbol {
    /// Constructs a new Symbol with a statically set price
    pub fn new_oneshot(
        price: Arc<(AtomicUsize, AtomicUsize)>, is_fx: bool, decimals: usize
    ) -> Symbol {
        Symbol {
            input_stream: None,
            client_stream: None,
            metadata: SymbolData {
                is_fx: is_fx,
                decimal_precision: decimals,
            },
            price: price,
        }
    }

    /// Returns `true` if this symbol is an exchange rate.
    pub fn is_fx(&self) -> bool {
        self.metadata.is_fx
    }

    /// Sends a `Tick` through the client stream.  This will block until the client consumes
    /// the tick.
    pub fn send_client(&mut self, t: Tick) {
        let sender = mem::replace(&mut self.client_stream, None)
            .expect("No client stream has been initialized for this symbol!");
        let new_sender = sender.send(t).wait().unwrap();
        mem::replace(&mut self.client_stream, Some(new_sender));
    }
}

/// A container that holds all data about prices and symbols.  Contains helper functions for
/// easily extracting data out and indexing efficiently.
pub struct Symbols {
    /// Holds the actual symbol data in a Vector.
    data: Vec<Symbol>,
    /// Matches the data's symbols to their index in the vector
    hm: HashMap<String, usize>,
}

/// Allow immutable indexing of the inner data Vector for high-speed internal data access
impl Index<usize> for Symbols {
    type Output = Symbol;

    fn index(&self, index: usize) -> &Self::Output {
        self.data.get(index).unwrap()
    }
}

/// Allow mutable indexing of the inner data Vector for high-speed internal data access
impl IndexMut<usize> for Symbols {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.data.get_mut(index).unwrap()
    }
}

/// Allow indexing the inner data `Vec` by `String` via looking up through the internal `HashMap`.
impl<'a> Index<&'a String> for Symbols {
    type Output = Symbol;

    fn index(&self, index: &'a String) -> &Self::Output {
        match self.hm.get(index) {
            Some(ix) => self.data.get(*ix).unwrap(),
            None => panic!("Attempted to get {} by String but can't find a match!", index),
        }
    }
}

impl Symbols {
    pub fn new() -> Symbols {
        Symbols {
            data: Vec::new(),
            hm: HashMap::new(),
        }
    }

    pub fn contains(&self, name: &String) -> bool {
        self.hm.contains_key(name)
    }

    pub fn add(&mut self, name: String, symbol: Symbol) {
        assert!(!self.contains(&name));
        self.data.push(symbol);
        let ix = self.data.len() - 1;
        self.hm.insert(name, ix);
    }
}

/// A simulated broker that is used as the endpoint for trading activity in backtests.
pub struct SimBroker {
    /// Contains all the accounts simulated by the SimBroker
    pub accounts: Arc<Mutex<HashMap<Uuid, Account>>>,
    /// A copy of the settings generated from the input HashMap
    pub settings: SimBrokerSettings,
    /// Contains the streams that yield `Tick`s for the SimBroker as well as data about
    /// the symbols and other metadata.
    symbols: Symbols,
    /// Priority queue that maintains that forms the basis of the internal ordered event loop.
    pq: BinaryHeap<QueueItem>,
    /// Timestamp of last price update received by broker
    timestamp: Arc<AtomicUsize>,
    /// A handle to the sender for the channel through which push messages are sent
    push_stream_handle: Option<Sender<BrokerResult>>,
    /// A handle to the receiver for the channel throgh which push messages are received
    push_stream_recv: Option<Receiver<BrokerResult>>,
    /// The CommandServer used for logging
    pub cs: CommandServer,
}

impl Broker for SimBroker {
    fn init(settings: HashMap<String, String>) -> Oneshot<Result<Self, BrokerError>> {
        let (c, o) = oneshot::<Result<Self, BrokerError>>();
        // this currently panics if you give it bad values...
        // TODO: convert FromHashmap to return a Result<SimbrokerSettings>
        let broker_settings = SimBrokerSettings::from_hashmap(settings);
        let cs = CommandServer::new(Uuid::new_v4(), "Simbroker");
        c.complete(Ok(SimBroker::new(broker_settings, cs)));

        o
    }

    fn get_ledger(&mut self, account_id: Uuid) -> Oneshot<Result<Ledger, BrokerError>> {
        let (complete, oneshot) = oneshot::<Result<Ledger, BrokerError>>();
        let account = self.get_ledger_clone(account_id);
        complete.complete(account);

        oneshot
    }

    fn list_accounts(&mut self) -> Oneshot<Result<HashMap<Uuid, Account>, BrokerError>> {
        let (complete, oneshot) = oneshot::<Result<HashMap<Uuid, Account>, BrokerError>>();
        let accounts;
        {
            let _accounts = self.accounts.lock().unwrap();
            accounts = _accounts.clone();
        }
        complete.complete(Ok(accounts));

        oneshot
    }

    fn execute(&mut self, action: BrokerAction) -> PendingResult {
        let (complete, oneshot) = oneshot::<BrokerResult>();

        // TODO

        oneshot
    }

    fn get_stream(&mut self) -> Result<Box<Stream<Item=BrokerResult, Error=()> + Send>, BrokerError> {
        if self.push_stream_recv.is_none() {
            // TODO: Enable multiple handles to be taken?
            return Err(BrokerError::Message{
                message: "You already took a handle to the push stream and can't take another.".to_string()
            })
        }

        Ok(self.push_stream_recv.take().unwrap().boxed())
    }

    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()> + Send>, BrokerError> {
        unimplemented!();
    }
}

impl SimBroker {
    pub fn new(settings: SimBrokerSettings, cs: CommandServer) -> SimBroker {
        let mut accounts = HashMap::new();
        // create with one account with the starting balance.
        let account = Account {
            uuid: Uuid::new_v4(),
            ledger: Ledger::new(settings.starting_balance),
            live: false,
        };
        accounts.insert(Uuid::new_v4(), account);
        // TODO: Make sure that 0 is the right buffer size for this channel
        let (tx, rx) = channel::<BrokerResult>(0);

        SimBroker {
            accounts: Arc::new(Mutex::new(accounts)),
            settings: settings,
            symbols: Symbols::new(),
            pq: BinaryHeap::new(),
            timestamp: Arc::new(AtomicUsize::new(0)),
            push_stream_handle: Some(tx),
            push_stream_recv: Some(rx),
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
    fn init_sim_loop(&mut self) {
        // continue looping while the priority queue has new events to simulate
        while let Some(item) = self.pq.pop() {
            match item.unit {
                WorkUnit::Tick(symbol_id, tick) => {
                    // TODO: Check to see if this does a copy and if it does, fine a way to eliminate it
                    let mut inner_symbol = &mut self.symbols[symbol_id];
                    // send the tick through the client stream, blocking until it is consumed by the client.
                    inner_symbol.send_client(tick);
                },
                WorkUnit::PendingAction(future, action) => {
                    // process the message and re-insert the response into the queue
                    let res = self.exec_action(&action, item.timestamp);
                    // calculate when the response would be recieved by the client
                    let execution_delay = self.settings.get_delay(&action);
                    let res_time = item.timestamp + self.settings.ping_ns + execution_delay;
                    let item = QueueItem {
                        timestamp: res_time,
                        unit: WorkUnit::Response(future, res),
                    };
                    self.pq.push(item);
                },
                WorkUnit::Response(future, res) => {
                    // send the push message through the channel, blocking until it's consumed by the client.
                    self.push_msg(res);
                }
            }
        }

        // if we reach here, that means we've run out of ticks to simulate from the input streams.
        self.cs.notice(None, "All input tickstreams have ran out of ticks; internal simulation loop stopped.");
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
    fn exec_action(&mut self, cmd: &BrokerAction, timestamp: u64) -> BrokerResult {
        match cmd {
            &BrokerAction::Ping => {
                Ok(BrokerMessage::Pong{time_received: timestamp})
            },
            &BrokerAction::TradingAction{ref action} => {
                match action {
                    &TradingAction::MarketOrder{account, ref symbol, long, size, stop, take_profit, max_range} => {
                        unimplemented!(); // TODO
                    },
                    &TradingAction::MarketClose{uuid, size} => {
                        unimplemented!(); // TODO
                    }
                    &TradingAction::LimitOrder{account, ref symbol, long, size, stop, take_profit, entry_price} => {
                        unimplemented!(); // TODO
                    },
                    &TradingAction::LimitClose{uuid, size, exit_price} => {
                        unimplemented!(); // TODO
                    },
                    &TradingAction::ModifyPosition{uuid, stop, take_profit, entry_price} => {
                        unimplemented!(); // TODO
                    }
                }
            },
            &BrokerAction::Disconnect => unimplemented!(),
        }
    }

    /// Opens a position at the current market price with options for settings stop loss, or take profit.
    fn market_open(
        &mut self, account_id: Uuid, symbol: &String, long: bool, size: usize, stop: Option<usize>,
        take_profit: Option<usize>, max_range: Option<f64>, timestamp: u64
    ) -> BrokerResult {
        let opt = self.get_price(symbol);
        if opt.is_none() {
            return Err(BrokerError::NoSuchSymbol)
        }
        let (bid, ask) = opt.unwrap();
        let is_fx = self.symbols[symbol].is_fx();

        let cur_price = if long { ask } else { bid };

        let pos = Position {
            creation_time: timestamp,
            symbol: symbol.clone(),
            size: size,
            price: Some(cur_price),
            long: long,
            stop: stop,
            take_profit: take_profit,
            execution_time: Some(timestamp + self.settings.execution_delay_ns as u64),
            execution_price: Some(cur_price),
            exit_price: None,
            exit_time: None,
        };

        let open_cost = if is_fx {
            let primary_currency = &symbol[0..3];
            let base_rate = try!(self.get_base_rate(primary_currency));
            (size * base_rate) / self.settings.leverage
        } else {
            size / self.settings.leverage
        };

        let mut accounts = self.accounts.lock().unwrap();
        let account_ = accounts.entry(account_id);
        match account_ {
            Entry::Occupied(mut occ) => {
                let mut account = occ.get_mut();
                account.ledger.open_position(pos, open_cost)
            },
            Entry::Vacant(_) => {
                Err(BrokerError::NoSuchAccount)
            }
        }
    }

    /// Dumps the SimBroker state to a file that can be resumed later.
    fn dump_to_file(&mut self, filename: &str) {
        unimplemented!(); // TODO
    }

    /// Used for Forex exchange rate conversions.  The cost to open a position is determined
    /// by the exchange rate between the base currency and the primary currency of the pair.
    ///
    /// Gets the conversion rate (in pips) between the base currency of the simbroker and
    /// the supplied currency.  If the base currency is USD and AUD is provided, the exchange
    /// rate for AUD/USD will be returned.  Returns Err if we lack the data to do that.
    fn get_base_rate(&self, symbol: &str) -> Result<usize, BrokerError> {
        if !self.settings.fx {
            return Err(BrokerError::Message{
                message: String::from("Can only convert to base rate when in FX mode.")
            });
        }

        let base_currency = &self.settings.fx_base_currency;
        let base_pair = format!("{}{}", symbol, base_currency);

        // TODO: handle reverses (USDEUR)

        let (bid, ask) = match self.get_price(&base_pair) {
            Some(price) => price,
            None => {
                return Err(BrokerError::NoDataAvailable);
            },
        };

        Ok(ask)
    }

    /// Returns the worth of a position in units of base currency.
    fn get_position_value(&self, pos: &Position) -> Result<usize, BrokerError> {
        unimplemented!(); // TODO
    }

    /// Sets the price for a symbol.  If no Symbol currently exists with that designation, a new one
    /// will be initialized with a static price.
    fn oneshot_price_set(
        &mut self, name: String, price: (usize, usize), is_fx: bool, decimal_precision: usize,
    ) {
        let (bid, ask) = price;
        if is_fx {
            assert_eq!(name.len(), 6);
        }

        // insert new entry into `self.prices` or update if one exists
        if self.symbols.contains(&name) {
            let (ref bid_atom, ref ask_atom) = *self.symbols[&name].price;
            bid_atom.store(bid, Ordering::Relaxed);
            ask_atom.store(ask, Ordering::Relaxed);
        } else {
            let bid_atom = AtomicUsize::new(bid);
            let ask_atom = AtomicUsize::new(ask);
            let atom_tuple = (bid_atom, ask_atom);

            let symbol = Symbol::new_oneshot(Arc::new(atom_tuple), is_fx, decimal_precision);
            self.symbols.add(name, symbol);
        }
    }

    /// Returns a clone of an account's ledger or an error if it doesn't exist.
    pub fn get_ledger_clone(&mut self, account_uuid: Uuid) -> Result<Ledger, BrokerError> {
        let accounts = self.accounts.lock().unwrap();
        match accounts.get(&account_uuid) {
            Some(acct) => Ok(acct.ledger.clone()),
            None => Err(BrokerError::Message{
                message: "No account exists with that UUID.".to_string()
            }),
        }
    }

    /// Called each tick to check if any pending positions need opening or closing.
    fn tick_positions(
        symbol: String,
        sender_handle: &UnboundedSender<Result<BrokerMessage, BrokerError>>,
        accounts_mutex: Arc<Mutex<HashMap<Uuid, Account>>>,
        price_arc: Arc<(AtomicUsize, AtomicUsize)>,
        timestamp: u64
    ) {
        let mut accounts = accounts_mutex.lock().unwrap();
        for (acct_id, mut acct) in accounts.iter_mut() {
            let (ref bid_atom, ref ask_atom) = *price_arc;
            let (bid, ask) = (bid_atom.load(Ordering::Relaxed), ask_atom.load(Ordering::Relaxed));
            let mut satisfied_pendings = Vec::new();

            for (pos_id, pos) in &acct.ledger.pending_positions {
                let satisfied = pos.is_open_satisfied(bid, ask);
                // market conditions have changed and this position should be opened
                if pos.symbol == symbol && satisfied.is_some() {
                    satisfied_pendings.push( (*pos_id, satisfied) );
                }
            }

            // fill all the satisfied pending positions
            for (pos_id, price_opt) in satisfied_pendings {
                let mut pos = acct.ledger.pending_positions.remove(&pos_id).unwrap();
                pos.execution_time = Some(timestamp);
                pos.execution_price = price_opt;
                // TODO: Adjust account balance and stats
                acct.ledger.open_positions.insert(pos_id, pos.clone());
                // send push message with notification of fill
                let _ = sender_handle.send(
                    Ok(BrokerMessage::PositionOpened{
                        position_id: pos_id, position: pos, timestamp: timestamp
                    })
                );
            }

            let mut satisfied_opens = Vec::new();
            for (pos_id, pos) in &acct.ledger.open_positions {
                let satisfied = pos.is_close_satisfied(bid, ask);
                // market conditions have changed and this position should be closed
                if pos.symbol == symbol && satisfied.is_some() {
                    satisfied_opens.push( (*pos_id, satisfied) );
                }
            }

            // close all the satisfied open positions
            for (pos_id, closure) in satisfied_opens {
                let (close_price, closure_reason) = closure.unwrap();
                let mut pos = acct.ledger.pending_positions.remove(&pos_id).unwrap();
                pos.exit_time = Some(timestamp);
                pos.exit_price = Some(close_price);
                // TODO: Adjust account balance and stats
                acct.ledger.closed_positions.insert(pos_id, pos.clone());
                // send push message with notification of close
                let _ = sender_handle.send(
                    Ok(BrokerMessage::PositionClosed{
                        position_id: pos_id, position: pos, reason: closure_reason, timestamp: timestamp
                    })
                );
            }
        }
    }

    /// Registers a data source into the SimBroker.  Ticks from the supplied generator will be
    /// used to upate the SimBroker's internal prices and transmitted to connected clients.
    pub fn register_tickstream(
        &mut self, symbol: String, raw_tickstream: UnboundedReceiver<Tick>, is_fx: bool, decimal_precision: usize
    ) -> Result<(), String> {
        unimplemented!();
        // // wire the tickstream into the SimBroker internals
        // let price_arc = self.prices.entry(symbol.clone())
        //     .or_insert_with(|| {
        //         Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)))
        //     }).clone();

        // // wire the tickstream so that the broker updates its own prices before sending the
        // // price updates off to the client
        // let accounts_clone = self.accounts.clone();
        // let push_handle = self.get_push_handle();
        // let timestamp_atom = self.timestamp.clone();
        // let wired_tickstream = wire_tickstream(
        //     symbol.clone(), raw_tickstream, accounts_clone, timestamp_atom, push_handle
        // );
        // self.tick_receivers.insert(symbol, wired_tickstream);
        // Ok(())
    }

    /// Returns the current price for a given symbol or None if the SimBroker
    /// doensn't have a price.
    pub fn get_price(&self, name: &String) -> Option<(usize, usize)> {
        if !self.symbols.contains(name) {
            return None;
        }

        let (ref atom_bid, ref atom_ask) = *self.symbols[name].price;
        Some((atom_bid.load(Ordering::Relaxed), atom_ask.load(Ordering::Relaxed)))
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp.load(Ordering::Relaxed) as u64
    }
}

// /// Called during broker initialization.  Takes a stream of live ticks from the backtester
// /// and uses it to power its own prices, returning a Stream that can be passed off to
// /// a client to serve as its price feed.
// fn wire_tickstream(
//     is_fx: bool, decimal_precision: usize,
//     price_arc: Arc<(AtomicUsize, AtomicUsize)>, symbol: String, tickstream: UnboundedReceiver<Tick>,
//     accounts: Arc<Mutex<HashMap<Uuid, Account>>>, timestamp_atom: Arc<AtomicUsize>,
//     push_stream_handle: UnboundedSender<Result<BrokerMessage, BrokerError>>
// ) -> InputTickstream {
//     let wired_stream = tickstream.map(move |t| {
//         let (ref bid_atom, ref ask_atom) = *price_arc;

//         // convert the tick's prices to pips and store
//         bid_atom.store(t.bid, Ordering::Relaxed);
//         ask_atom.store(t.ask, Ordering::Relaxed);
//         println!("Prices successfully wired into atomics");
//         // store timestamp
//         (*timestamp_atom).store(t.timestamp as usize, Ordering::Relaxed);

//         // check if any positions need to be opened/closed due to this tick
//         SimBroker::tick_positions(symbol.clone(), &push_stream_handle, accounts.clone(), price_arc.clone(), t.timestamp as u64);
//         t
//     }).boxed();

//     InputTickstream {
//         stream: wired_stream,
//         is_fx: is_fx,
//         decimal_precision: decimal_precision,
//     }
// }

/// It should be an error to try to subscribe to a symbol that the SimBroker doesn't keep track of.
#[test]
fn sub_ticks_err() {
    let settings = SimBrokerSettings::default();

    let mut sim_b = SimBroker::new(settings, CommandServer::new(Uuid::new_v4(), "SimBroker Test"));
    let stream = sim_b.sub_ticks("TEST".to_string());
    assert!(stream.is_err());
}

/// How long it takes to unwrap the sender, send a message, and re-store the sender.
#[bench]
fn send_push_message(b: &mut test::Bencher) {
    let settings = SimBrokerSettings::default();
    let mut sim_b = SimBroker::new(settings, CommandServer::new(Uuid::new_v4(), "SimBroker Test"));
    let receiver = sim_b.get_stream().unwrap();
    thread::spawn(move ||{
        for _ in receiver.wait() {

        }
    });

    b.iter(|| {
        sim_b.push_msg(Ok(BrokerMessage::Success))
    })
}

/// Ticks sent to the SimBroker should be re-broadcast to the client.
#[test]
fn tick_retransmission() {
    use std::sync::mpsc;

    use futures::Future;

    use data::random_reader::RandomReader;
    use data::TickGenerator;
    use backtest::{FastMap, BacktestCommand};

    // create the SimBroker
    let symbol = "TEST".to_string();
    let settings = SimBrokerSettings::default();
    let mut sim_b = SimBroker::new(settings, CommandServer::new(Uuid::new_v4(), "SimBroker Test"));
    let msg_stream = sim_b.get_stream();

    // create a random tickstream and register it to the SimBroker
    let mut gen = RandomReader::new(symbol.clone());
    let map = Box::new(FastMap {delay_ms: 1});
    let (tx, rx) = mpsc::sync_channel(5);
    let tick_stream = gen.get(map, rx);
    let res = sim_b.register_tickstream(symbol.clone(), tick_stream.unwrap(), false, 0);
    assert!(res.is_ok());

    // subscribe to ticks from the SimBroker for the test pair
    let subbed_ticks = sim_b.sub_ticks(symbol).unwrap();
    let (c, o) = oneshot::<Vec<Tick>>();
    thread::spawn(move || {
        let res = subbed_ticks
            .wait()
            .take(10)
            .map(|t| t.unwrap() )
            .collect();
        // signal once we've received all the ticks
        c.complete(res);
    });

    // start the random tick generator
    let _ = tx.send(BacktestCommand::Resume);
    // block until we've received all awaited ticks
    let res = o.wait().unwrap();
    assert_eq!(res.len(), 10);
}

#[test]
fn position_opening_closing_modification() {
    use futures::Future;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut sim = SimBroker::init(HashMap::new()).wait().unwrap().unwrap();

    let price = (0999, 1001);
    sim.oneshot_price_set(String::from("TEST"), price, false, 4);
    // TODO
}

#[test]
fn dynamic_base_rate_conversion() {
    use std::default::Default;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut settings = SimBrokerSettings::default();
    settings.fx_accurate_pricing = true;
    let mut sim = SimBroker::new(settings, cs);

    // wire tickstreams into the broker
    let (base_tx, base_rx) = unbounded::<Tick>();
    let base_pair    = String::from("EURUSD");
    let (foreign_tx, foreign_rx) = unbounded::<Tick>();
    let foreign_pair = String::from("EURJPY");
    sim.register_tickstream(base_pair.clone(), base_rx, true, 4);
    sim.register_tickstream(foreign_pair.clone(), foreign_rx, true, 4);

    base_tx.send(Tick {
        timestamp: 1,
        bid: 106143,
        ask: 106147
    });
    foreign_tx.send(Tick {
        timestamp: 2,
        bid: 1219879,
        ask: 1219891,
    });
    assert_eq!((106141, 106147), sim.get_price(&base_pair).unwrap());
    assert_eq!((1219879, 1219891), sim.get_price(&foreign_pair).unwrap());
    // TODO: Test reverses (EURUSD and USDEUR)
}

#[test]
fn oneshot_price_setting() {
    use futures::Future;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut sim = SimBroker::init(HashMap::new()).wait().unwrap().unwrap();

    let price = (0999, 1001);
    let sym = String::from("TEST");
    sim.oneshot_price_set(sym.clone(), price, false, 4);
    assert_eq!(price, sim.get_price(&sym).unwrap());
}

#[test]
fn oneshot_base_rate_conversion() {
    use futures::Future;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut sim = SimBroker::init(HashMap::new()).wait().unwrap().unwrap();
    // TODO
}

#[bench]
fn mutex_lock_unlock(b: &mut test::Bencher) {
    let amtx = Arc::new(Mutex::new(0));
    b.iter(move || {
        let _ = amtx.lock();
    })
}

#[bench]
fn small_string_hashmap_lookup(b: &mut test::Bencher) {
    let mut hm = HashMap::new();
    hm.insert(String::from("key1"), String::from("val1"));
    hm.insert(String::from("key2"), String::from("val2"));
    hm.insert(String::from("key3"), String::from("val3"));
    hm.insert(String::from("key4"), String::from("val4"));
    hm.insert(String::from("key5"), String::from("val5"));

    let lookup_key = String::from("key4");
    b.iter(|| hm.get(&lookup_key))
}
