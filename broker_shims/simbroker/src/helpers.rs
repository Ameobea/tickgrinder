//! Contains all the helper objects and functions for the SimBroker.  Helpers inlclude objects to hold
//! data about the SimBroker's state and their corresponding functions and trait implementations.

use serde_json;

use std::intrinsics::unlikely;
use std::slice::{Iter, IterMut};
use std::fmt::{self, Formatter, Debug};

use super::*;
use superlog::CacheAction;

/// Returns a struct given the struct's field:value pairs in a `HashMap`.  If the provided `HashMap`
/// doesn't contain a field, then the default is used.
pub trait FromHashmap<T> : Default {
    fn from_hashmap(hm: HashMap<String, String>) -> T;
}

/// Settings for the simulated broker that determine things like trade fees,estimated slippage, etc.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
// procedural macro is defined in the `from_hashmap` crate found in the util directory's root.
#[derive(FromHashmap)]
pub struct SimBrokerSettings {
    pub starting_balance: usize,
    /// How many nanoseconds ahead the broker is to the client
    pub ping_ns: u64,
    /// How many nanoseconds between when the broker receives an order and executes it
    pub execution_delay_ns: u64,
    /// Buying power is leverage * balance
    pub leverage: usize,
    /// Contains the JSON-serialized version of the Vec<(String, TickGenerators)> containing
    /// symbol-gen pairs used to create tickstreams to power the broker.
    pub tickstreams: String,
    /// `true` if this simbroker is simulating a forex borker
    pub fx: bool,
    /// Base currency in which the SimBroker is funded.  Should be in the lowest division of that
    /// currency available (e.g. cents).
    pub fx_base_currency: String,
    /// For forex, the amount of units of currency in one lot.
    pub fx_lot_size: usize,
    /// For forex, if true, calculates accurate position values by dynamically converting to the base
    /// currency.  If false, the rate must be set before broker initialization.
    pub fx_accurate_pricing: bool,
}

impl Default for SimBrokerSettings {
    fn default() -> SimBrokerSettings {
        let tickstreams = serde_json::to_string(&vec![("TEST", TickGenerators::RandomReader, false, 0)]).unwrap();

        SimBrokerSettings {
            starting_balance: 50 * 1000 * 100, // $50,000
            ping_ns: 0,
            execution_delay_ns: 0,
            leverage: 50,
            tickstreams: tickstreams,
            fx: true,
            fx_base_currency: String::from("USD"),
            fx_lot_size: 1000,
            fx_accurate_pricing: false,
        }
    }
}

impl SimBrokerSettings {
    /// Returns the delay in ns for executing a particular `BrokerAction`.
    pub fn get_delay(&self, action: &BrokerAction) -> u64 {
        // TODO: implement delays for each of the `BrokerAction`s
        self.execution_delay_ns
    }
}

#[test]
fn simbroker_settings_hashmap_population() {
    let mut hm = HashMap::new();
    hm.insert(String::from("ping_ns"), String::from("2000"));
    let settings = SimBrokerSettings::from_hashmap(hm);
    assert_eq!(settings.ping_ns, 2000);
}

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
pub struct PendingAction {
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
pub enum WorkUnit {
    /// Simulates trading events triggering a new Tick for a particular symbol on the broker.
    /// Allocating Strings for each tick would be way too expensive, so indexes of
    /// managed ticks are used instead.
    NewTick(usize, Tick),
    /// Simulates a Tick arriving at the client
    ClientTick(usize, Tick),
    /// Simulates an action being processed by the Broker (after processing time).
    ActionComplete(Complete<BrokerResult>, BrokerAction),
    /// Simulates a message from the broker being received by a client.
    Response(Complete<BrokerResult>, BrokerResult),
}

impl PartialEq for WorkUnit {
    fn eq(&self, other: &WorkUnit) -> bool {
        match *self {
            WorkUnit::NewTick(self_ix, self_tick) => {
                match *other {
                    WorkUnit::NewTick(other_ix, other_tick) => {
                        self_ix == other_ix && self_tick == other_tick
                    },
                    _ => false,
                }
            },
            WorkUnit::ClientTick(self_ix, self_tick) => {
                match *other {
                    WorkUnit::ClientTick(other_ix, other_tick) => {
                        self_ix == other_ix && self_tick == other_tick
                    },
                    _ => false,
                }
            },
            WorkUnit::ActionComplete(_, ref self_action) => {
                match *other {
                    WorkUnit::ActionComplete(_, ref other_action) => {
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

impl Debug for WorkUnit {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            WorkUnit::NewTick(self_ix, self_tick) => {
                write!(f, "NewTick({}, {:?})", self_ix, self_tick)
            },
            WorkUnit::ClientTick(self_ix, self_tick) => {
                write!(f, "ClientTick({}, {:?})", self_ix, self_tick)
            },
            WorkUnit::ActionComplete(_, ref self_action) => {
                write!(f, "ActionComplete(_, {:?})", self_action)
            },
            WorkUnit::Response(_, ref self_res) => {
                write!(f, "Response(_, {:?})", self_res)
            }
        }
    }
}

impl Eq for WorkUnit {}

/// A timestamped unit of data for the priority queue.
#[derive(PartialEq, Eq)]
pub struct QueueItem {
    pub timestamp: u64,
    pub unit: WorkUnit,
}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &QueueItem) -> Option<::std::cmp::Ordering> {
        // Returns the OPPOSITE of the actual order because the `BinaryHeap` is a MAX-heap and
        // we want to pop off the events with the smallest timestamps first.
        Some(other.timestamp.cmp(&self.timestamp))
    }
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
        // Returns the OPPOSITE of the actual order because the `BinaryHeap` is a MAX-heap and
        // we want to pop off the events with the smallest timestamps first.
        other.timestamp.cmp(&self.timestamp)
    }
}

pub struct Symbol {
    pub name: String,
    /// The input stream that yields the ticks converted into an iterator.
    pub input_iter: Option<Box<Iterator<Item=Result<Tick, ()>>>>,
    /// The tx-side of the tickstream that's handed off to the client.
    pub client_sender: Option<Sender<Tick>>,
    /// The stream that is handed off to the client.  Only yields `Tick`s when the order
    /// of events dictates it inside the internal simulation loop.
    pub client_receiver: Option<Box<Stream<Item=Tick, Error=()> + Send>>,
    /// Contains some information about the symbol that the ticks represent
    pub metadata: SymbolData,
    /// Broker's view of prices in pips, determined by the `tick_receiver`s
    pub price: (usize, usize),
    /// The next tick for this stream; used for ordering in SimBroker's internal queue
    pub next_tick: Option<Tick>,
}

impl Symbol {
    /// Constructs a new Symbol with a statically set price
    pub fn new_oneshot(price: (usize, usize), is_fx: bool, decimals: usize, name: String) -> Symbol {
        Symbol {
            name: name,
            input_iter: None,
            client_sender: None,
            client_receiver: None,
            metadata: SymbolData {
                is_fx: is_fx,
                decimal_precision: decimals,
            },
            price: price,
            next_tick: None,
        }
    }

    pub fn new_from_stream(stream: Box<Stream<Item=Tick, Error=()>>, is_fx: bool, decimals: usize, name: String) -> Symbol {
        // TODO: Make sure that 0 is the right buffer size to use
        let (client_tx, client_rx) = channel(0);
        let mut iter = stream.wait();
        let future_tick = iter.next().unwrap().unwrap();

        Symbol {
            name: name,
            input_iter: Some(Box::new(iter)),
            client_sender: Some(client_tx),
            client_receiver: Some(client_rx.boxed()),
            metadata: SymbolData {
                is_fx: is_fx,
                decimal_precision: decimals,
            },
            price: (0, 0),
            next_tick: Some(future_tick),
        }
    }

    /// Returns `true` if this symbol is an exchange rate.
    pub fn is_fx(&self) -> bool {
        self.metadata.is_fx
    }

    /// Sends a `Tick` through the client stream.  This will block until the client consumes
    /// the tick.
    pub fn send_client(&mut self, t: Tick) {
        let sender = mem::replace(&mut self.client_sender, None)
            .expect("No client stream has been initialized for this symbol!");
        let new_sender = sender.send(t).wait().expect("Client stream is gone; probably due to shutdown.");
        mem::replace(&mut self.client_sender, Some(new_sender));
    }

    /// Returns (bid, ask, decimal_precision)
    pub fn get_price(&self) -> (usize, usize, usize) {
        (self.price.0, self.price.1, self.metadata.decimal_precision)
    }

    /// Returns the next element from the internal iterator
    pub fn next(&mut self) -> Option<Result<Tick, ()>> {
        let iter = self.input_iter.as_mut().expect("No input iterator for that symbol!");
        iter.next()
    }
}

/// A container that holds all data about prices and symbols.  Contains helper functions for
/// easily extracting data out and indexing efficiently.
pub struct Symbols {
    /// Holds the actual symbol data in a Vector.
    data: Vec<Symbol>,
    /// Matches the data's symbols to their index in the vector
    hm: HashMap<String, usize>,
    /// Clone of the SimBroker's `CommandServer`
    cs: CommandServer,
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

/// Allow immutable indexing of the inner data `Vec` by `String` via looking up through the internal `HashMap`.
impl<'a> Index<&'a String> for Symbols {
    type Output = Symbol;

    fn index(&self, index: &'a String) -> &Self::Output {
        match self.hm.get(index) {
            Some(ix) => self.data.get(*ix).unwrap(),
            None => panic!("Attempted to get {} by String but can't find a match!", index),
        }
    }
}

/// Allow mutable indexing of the inner data `Vec` by `String` via looking up through the internal `HashMap`.
impl<'a> IndexMut<&'a String> for Symbols {
    fn index_mut(&mut self, index: &'a String) -> &mut Self::Output {
        match self.hm.get(index) {
            Some(ix) => self.data.get_mut(*ix).unwrap(),
            None => panic!("Attempted to get {} by String but can't find a match!", index),
        }
    }
}

impl Symbols {
    pub fn new(cs: CommandServer) -> Symbols {
        Symbols {
            data: Vec::new(),
            hm: HashMap::new(),
            cs: cs,
        }
    }

    pub fn get_index(&self, name: &String) -> Option<usize> {
        self.hm.get(name).map(|r| *r)
    }

    pub fn contains(&self, name: &String) -> bool {
        self.hm.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Attempts to add a `Symbol` to the list of managed symbols.
    pub fn add(&mut self, name: String, symbol: Symbol) -> BrokerResult {
        if self.contains(&name) {
            return Err(BrokerError::Message{
                message: String::from("A tickstream already exists for that symbol!"),
            });
        }

        self.data.push(symbol);
        let ix = self.data.len() - 1;
        self.hm.insert(name, ix);
        Ok(BrokerMessage::Success)
    }

    pub fn iter(&self) -> Iter<Symbol> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<Symbol> {
        self.data.iter_mut()
    }

    /// Returns the index and `Tick` of the future tick with the smallest timestamp.
    pub fn next_tick(&mut self) -> Option<(usize, Tick)> {
        let mtick2;
        let mindex2;
        // because I want you to drop that borrow THIS MUCH
        {
            // find the tick with the lowest future timestamp from in the queue
            let (mut mindex, mut mtick) = (0, self.data[0].next_tick.as_ref());
            for i in 1..self.data.len() {
                if self.data[i].next_tick.is_some() {
                    let t = self.data[i].next_tick.as_ref().unwrap();
                    if mtick.is_none() || mtick.unwrap().timestamp > (*t).timestamp {
                        mindex = i;
                        mtick = Some(t);
                    }
                }
            }

            mtick2 = mtick.map(|tick_ref| tick_ref.clone());
            mindex2 = mindex;
        }

        // Get the next future tick for that symbol and return the old one
        let next_future_opt = self.data[mindex2].next().map(|tick_res| tick_res.unwrap()).clone();
        self.data[mindex2].next_tick = next_future_opt;

        mtick2.map(|t_ref| (mindex2, t_ref))
    }
}

/// Wrapper around the `BinaryHeap` that forms the basis of the priority queue for the simulation loop.
/// Maintains state for the largest and smallest timestamps of all elements in the queue.
pub struct SimulationQueue {
    /// The `BinaryHeap` itself, forming the core of the priority queue
    pub q: BinaryHeap<QueueItem>,
}

impl SimulationQueue {
    /// Creates a new `SimulationQueue` but doesn't initialize.  Not usable until after initialization.
    pub fn new() -> SimulationQueue {
        SimulationQueue {
            q: BinaryHeap::new(),
        }
    }

    /// Initializes the queue with values from the tickstreams contained in the `Symbols` object.  This
    /// should be called directly before starting the simulation loop.
    pub fn init(&mut self, symbols: &mut Symbols) {
        // Add n+1 ticks to the queue where n is the number of symbols in `Symbols`.
        // update min and max values manually
        for _ in 0..symbols.len() {
            let (ix, tick) = symbols.next_tick().unwrap();
            self.push(QueueItem {
                timestamp: tick.timestamp as u64,
                unit: WorkUnit::NewTick(ix, tick)
            });
        }
    }

    pub fn push(&mut self, item: QueueItem) {
        self.q.push(item)
    }

    pub fn pop(&mut self) -> Option<QueueItem> {
        self.q.pop()
    }

    /// Convenience function to push the next future tick into the queue.
    pub fn push_next_tick(&mut self, symbols: &mut Symbols) {
        match symbols.next_tick() {
            Some((ix, tick)) => self.push(QueueItem {
                timestamp: tick.timestamp as u64,
                unit: WorkUnit::NewTick(ix, tick),
            }),
            None => (),
        }
    }
}

/// The units stored in the cache; contains the position and some data to easily locate it in the main HashMap.
#[derive(Debug)]
pub struct CachedPosition {
    pub pos_uuid: Uuid,
    pub acct_uuid: Uuid,
    pub pos: Position,
}

/// All pending and open positions for a symbol
pub struct Positions {
    /// pending positions
    pub pending: Vec<CachedPosition>,
    /// open positions
    pub open: Vec<CachedPosition>,
}

impl Positions {
    pub fn new() -> Positions {
        Positions {
            pending: Vec::new(),
            open: Vec::new(),
        }
    }
}

/// Contains all of the accounts managed by the `SimBroker`.  Includes helper fields and methods for
/// position/order management during the simulation loop.
pub struct Accounts {
    /// The main HashMap containing all the accounts linked with their Uuids
    pub data: HashMap<Uuid, Account>,
    /// Contains copies of all pending and open positions for all accounts along with the account's Uuid
    pub positions: Vec<Positions>,
    pub logger: SuperLogger,
}

impl Accounts {
    pub fn new(logger: SuperLogger) -> Accounts {
        Accounts {
            data: HashMap::new(),
            positions: Vec::new(),
            logger: logger,
        }
    }

    /// Called every time a tickstream is registered to the Symbols struct; allocates new vectors
    /// to hold copies of positions for that symbol.
    pub fn add_symbol(&mut self) {
        self.positions.push(Positions::new());
    }

    pub fn insert(&mut self, k: Uuid, v: Account) -> Option<Account> {
        self.data.insert(k, v)
    }

    pub fn entry(&mut self, k: Uuid) -> Entry<Uuid, Account> {
        self.data.entry(k)
    }

    pub fn get(&mut self, k: &Uuid) -> Option<&Account> {
        self.data.get(k)
    }

    pub fn get_mut(&mut self, k: &Uuid) -> Option<&mut Account> {
        self.data.get_mut(k)
    }

    /// This is called when a new order is placed, indicating that it should be added to the pending cache.
    pub fn order_placed(&mut self, order: &Position, order_uuid: Uuid, account_uuid: Uuid) {
        let cached_pos = CachedPosition {
            pos_uuid: order_uuid,
            acct_uuid: account_uuid,
            pos: order.clone(),
        };
        self.logger.cache_log(CacheAction::OrderPlaced, account_uuid, order_uuid, order);
        self.positions[order.symbol_id].pending.push(cached_pos);
    }

    /// This is called when a pending position is manually modified but not closed, indicating that its cache
    /// value should be updated to the new supplied version.
    pub fn order_modified(&mut self, updated_order: &Position, supplied_uuid: Uuid) {
        for &mut CachedPosition { pos_uuid, acct_uuid, ref mut pos } in &mut self.positions[updated_order.symbol_id].pending {
            if pos_uuid == supplied_uuid {
                self.logger.cache_log(CacheAction::OrderModified{old_order: pos}, acct_uuid, pos_uuid, updated_order);
                *pos = updated_order.clone();
            }
        }
    }

    /// Called when an order is cancelled, causing it to be removed from the pending cache without being added to
    /// the open cache.
    pub fn order_cancelled(&mut self, cancelled_uuid: Uuid, symbol_ix: usize) {
        for i in 0..self.positions[symbol_ix].pending.len() {
            let pos_uuid = self.positions[symbol_ix].pending[i].pos_uuid;
            if pos_uuid == cancelled_uuid {
                let removed = self.positions[symbol_ix].pending.remove(i);
                self.logger.cache_log(CacheAction::OrderCancelled, removed.acct_uuid, cancelled_uuid, &removed.pos);
                return;
            }
        }

        panic!("We were told that an order was cancelled, but we couldn't find that order in the cache!");
    }

    /// This is called when a new position is opened manually, indicating that it should be removed from the pending
    /// cache and added to the open cache.
    pub fn position_opened(&mut self, pos: &Position, pos_uuid: Uuid) {
        assert!(pos.execution_time.is_some());
        assert!(pos.execution_price.is_some());
        let mut removed_pos = None;
        let mut i = 0;
        { // borrow-b-gone
            let mut pending_cache = &mut self.positions[pos.symbol_id].pending;
            for _ in 0..pending_cache.len() {
                if pending_cache[i].pos_uuid == pos_uuid {
                    // remove the position from the pending cache and add it to the open cache
                    let mut cached_pos = pending_cache.remove(i);
                    cached_pos.pos = pos.clone();
                    cached_pos.pos_uuid = pos_uuid;
                    self.logger.cache_log(CacheAction::OrderFilled, cached_pos.acct_uuid, cached_pos.pos_uuid, &cached_pos.pos);
                    removed_pos = Some(cached_pos);
                    break;
                }
                i += 1;
            }
        }

        // add the position to the open cache
        match removed_pos {
            Some(cached_pos) => self.positions[pos.symbol_id].open.push(cached_pos),
            None => panic!("`position_opened` was called, but there were no pending positions with the supplied uuid!"),
        }
    }

    /// This is called when a position is opened without a pre-existing pending order; it simply adds the position to
    /// the open cache without trying to close a pending position.
    pub fn position_opened_immediate(&mut self, pos: &Position, pos_uuid: Uuid, account_uuid: Uuid) {
        assert!(pos.execution_time.is_some());
        assert!(pos.execution_price.is_some());
        let cached_pos = CachedPosition {pos_uuid: pos_uuid, acct_uuid: account_uuid, pos: pos.clone()};
        self.logger.cache_log(CacheAction::PositionOpenedImmediate, account_uuid, pos_uuid, pos);
        self.positions[pos.symbol_id].open.push(cached_pos);
    }

    /// This is called when an open position is modified in some way, indicating that its cached value should be changed.
    pub fn position_modified(&mut self, updated_pos: &Position, supplied_uuid: Uuid) {
        for &mut CachedPosition { pos_uuid, acct_uuid, ref mut pos } in &mut self.positions[updated_pos.symbol_id].open {
            if pos_uuid == supplied_uuid {
                self.logger.cache_log(CacheAction::PositionModified{old_pos: pos}, acct_uuid, pos_uuid, updated_pos);
                *pos = updated_pos.clone();
            }
        }
    }

    /// This is called when an open position is closed manually, indicating that it should be removed from the cache.
    pub fn position_closed(&mut self, pos: &Position, pos_uuid: Uuid) {
        let mut open_cache = &mut self.positions[pos.symbol_id].open;
        for i in 0..open_cache.len() {
            if open_cache[i].pos_uuid == pos_uuid {
                let removed_pos = open_cache.remove(i);
                self.logger.cache_log(CacheAction::PositionClosed, removed_pos.acct_uuid, removed_pos.pos_uuid, pos);
                return
            }
        }

        panic!("`position_closed` was called, but there were no open positions with the supplied uuid!");
    }
}

/// Given a price with a specified decimal precision, converts the price to one with
/// a different decimal precision, rounding if necessary.
pub fn convert_decimals(in_price: usize, in_decimals: usize, out_decimals: usize) -> usize {
    if in_decimals > out_decimals {
        in_price / (10usize.pow((in_decimals - out_decimals) as u32))
    } else if out_decimals > in_decimals {
        in_price * (10usize.pow((out_decimals - in_decimals) as u32))
    } else if unsafe{ unlikely(out_decimals == in_decimals) } {
        in_price
    } else {
        unreachable!()
    }
}
