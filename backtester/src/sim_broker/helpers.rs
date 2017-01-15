//! Contains all the helper objects and functions for the SimBroker.  Helpers inlclude objects to hold
//! data about the SimBroker's state and their corresponding functions and trait implementations.

use std::intrinsics::unlikely;

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
pub struct QueueItem {
    pub timestamp: u64,
    pub unit: WorkUnit,
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
    /// The tx-side of the tickstream that's handed off to the client.
    pub client_sender: Option<Sender<Tick>>,
    /// The stream that is handed off to the client.  Only yields `Tick`s when the order
    /// of events dictates it inside the internal simulation loop.
    pub client_receiver: Option<Receiver<Tick>>,
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
            client_sender: None,
            client_receiver: None,
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
        let sender = mem::replace(&mut self.client_sender, None)
            .expect("No client stream has been initialized for this symbol!");
        let new_sender = sender.send(t).wait().unwrap();
        mem::replace(&mut self.client_sender, Some(new_sender));
    }

    /// Returns the price as a (bid, ask, decimals) tuple.
    pub fn get_price(&self) -> (usize, usize, usize) {
        let (ref bid_atom, ref ask_atom) = *self.price;
        (bid_atom.load(Ordering::Relaxed), ask_atom.load(Ordering::Relaxed), self.metadata.decimal_precision)
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

#[test]
fn decimal_conversion() {
    assert_eq!(1000, convert_decimals(0100, 2, 3));
    assert_eq!(0999, convert_decimals(9991, 4, 3));
    assert_eq!(0010, convert_decimals(1000, 3, 1));
    assert_eq!(0000, convert_decimals(0000, 8, 2));
    assert_eq!(0001, convert_decimals(0001, 3, 3));
}
