//! Contains all the helper objects and functions for the SimBroker.  Helpers inlclude objects to hold
//! data about the SimBroker's state and their corresponding functions and trait implementations.

use std::intrinsics::unlikely;
use std::slice::{Iter, IterMut};

use super::*;

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
    /// Simulates an action being received from a client.
    PendingAction(Complete<BrokerResult>, BrokerAction),
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
    /// The input stream that yields the ticks converted into an iterator.
    pub input_iter: Option<Box<Iterator<Item=Result<Tick, ()>>>>,
    /// The tx-side of the tickstream that's handed off to the client.
    pub client_sender: Option<Sender<Tick>>,
    /// The stream that is handed off to the client.  Only yields `Tick`s when the order
    /// of events dictates it inside the internal simulation loop.
    pub client_receiver: Option<Receiver<Tick>>,
    /// Contains some information about the symbol that the ticks represent
    pub metadata: SymbolData,
    /// Broker's view of prices in pips, determined by the `tick_receiver`s
    pub price: (usize, usize),
    /// The next tick for this stream; used for ordering in SimBroker's internal queue
    pub next_tick: Option<Tick>,
}

impl Symbol {
    /// Constructs a new Symbol with a statically set price
    pub fn new_oneshot(price: (usize, usize), is_fx: bool, decimals: usize) -> Symbol {
        Symbol {
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

    pub fn new_from_stream(stream: Box<Stream<Item=Tick, Error=()>>, is_fx: bool, decimals: usize) -> Symbol {
        // TODO: Make sure that 0 is the right buffer size to use
        let (client_tx, client_rx) = channel(0);
        let mut iter = stream.wait();
        let future_tick = iter.next().unwrap().unwrap();

        Symbol {
            input_iter: Some(Box::new(iter)),
            client_sender: Some(client_tx),
            client_receiver: Some(client_rx),
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
        let new_sender = sender.send(t).wait().unwrap();
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

    pub fn contains(&self, name: &String) -> bool {
        self.hm.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn add(&mut self, name: String, symbol: Symbol) {
        assert!(!self.contains(&name));
        self.data.push(symbol);
        let ix = self.data.len() - 1;
        self.hm.insert(name, ix);
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

#[test]
fn decimal_conversion() {
    assert_eq!(1000, convert_decimals(0100, 2, 3));
    assert_eq!(0999, convert_decimals(9991, 4, 3));
    assert_eq!(0010, convert_decimals(1000, 3, 1));
    assert_eq!(0000, convert_decimals(0000, 8, 2));
    assert_eq!(0001, convert_decimals(0001, 3, 3));
}

/// Make sure that the ordering of `QueueItem`s is reversed as it should be.
#[test]
fn reverse_event_ordering() {
    let item1 = QueueItem {
        timestamp: 5,
        unit: WorkUnit::Tick(0, Tick::null()),
    };
    let item2 = QueueItem {
        timestamp: 6,
        unit: WorkUnit::Tick(0, Tick::null()),
    };

    assert!(item2 < item1);
}
