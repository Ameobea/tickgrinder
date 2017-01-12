//! Holds definitions of the internal representations of trading objects and
//! abstractions for messages sent and received to brokers.

use std::collections::HashMap;

use uuid::Uuid;

use trading::trading_condition::*;
use trading::broker::*;

/// An account
#[derive(Clone, Debug)]
pub struct Account {
    pub uuid: Uuid,
    pub ledger: Ledger,
    pub live: bool, // false if a demo account
}

/// Any action that the platform can take using the broker
#[derive(Clone, Debug)]
pub enum BrokerAction {
    TradingAction{ action: TradingAction },
    /// Returns a Pong with the timestamp the broker received the message
    Ping,
}

/// A response from a broker indicating the result of an action.
#[derive(Clone, Debug)]
pub enum BrokerMessage {
    Success,
    Failure,
    Notice,
    PositionOpened{
        position_id: Uuid,
        position: Position,
        timestamp: u64
    },
    PositionClosed{
        position_id: Uuid,
        position: Position,
        reason: PositionClosureReason,
        timestamp: u64,
    },
    PositionModified{
        position_id: Uuid,
        position: Position,
        timestamp: u64,
    },
    Pong{time_received: u64},
}

#[derive(Clone, Debug)]
pub enum PositionClosureReason {
    StopLoss,
    TakeProfit,
    MarginCall,
    Expired,
    FillOrKill,
}

#[derive(Clone, Debug)]
pub enum BrokerError {
    Message{message: String},
    Unimplemented{message: String}, // the broker under the wrapper can't do what you asked it
    InsufficientBuyingPower,
    NoSuchPosition,
    NoSuchAccount,
    NoSuchSymbol,
}

/// The platform's internal representation of the current state of an account.
/// Contains information about past trades as well as current positions.
#[derive(Clone, Debug)]
pub struct Ledger {
    pub balance: f64,
    pub pending_positions: HashMap<Uuid, Position>,
    pub open_positions: HashMap<Uuid, Position>,
    pub closed_positions: HashMap<Uuid, Position>,
}

impl Ledger {
    pub fn new(starting_balance: f64) -> Ledger {
        Ledger {
            balance: starting_balance,
            pending_positions: HashMap::new(),
            open_positions: HashMap::new(),
            closed_positions: HashMap::new(),
        }
    }

    /// Opens the supplied position in the ledger.  Returns an error if there are insufficient funds
    /// in the ledger to open the position.
    ///
    /// `base_rate` is the exchange rate of the first currency of the pair to the bas currency
    /// If the position is EUR/JPY and the base currency is USD, then the base exchange rate
    /// would be that of EUR/USD.
    pub fn open_position(&mut self, pos: Position, base_rate: usize) -> BrokerResult {
        let uuid = Uuid::new_v4();
        let execution_time = pos.execution_time.unwrap();
        if pos.price.is_none() {
            return Err(BrokerError::Message{
                message: "The supplied position does not have an entry price.".to_string()
            })
        }

        let cost = (pos.price.unwrap() * pos.size as usize) as f64;
        if cost > self.balance as f64 {
            return Err(BrokerError::InsufficientBuyingPower)
        }
        self.balance -= cost;

        // TODO

        self.open_positions.insert(uuid, pos.clone());
        Ok(BrokerMessage::PositionOpened{
            position_id: uuid,
            position: pos,
            timestamp: execution_time,
        })
    }

    /// Completely closes the specified condition at the given price, crediting the account the
    /// funds yielded.
    pub fn close_position(&mut self, uuid: Uuid, base_rate: usize) -> BrokerResult {
        let res = self.open_positions.remove(&uuid);
        if res.is_none() {
            return Err(BrokerError::NoSuchPosition)
        }
        // TODO
        Ok(BrokerMessage::Success)
    }

    /// Increases or decreases the size of the specified position by the given amount.  Returns errors
    /// if the account doesn't have enough buying power to execute the action or if a position with
    /// the specified UUID doesn't exist.
    pub fn modify_position(&mut self, uuid: Uuid, base_rate: usize) -> BrokerResult {
        unimplemented!();
    }
}

/// Represents an opened, closed, or pending position on a broker.
#[derive(Clone, Debug)]
pub struct Position {
    pub creation_time: u64,
    pub symbol: String,
    pub size: u64,
    pub price: Option<usize>,
    pub long: bool,
    pub stop: Option<usize>,
    pub take_profit: Option<usize>,
    /// the price the position was actually executed
    pub execution_time: Option<u64>,
    /// the price the position was actually executed at
    pub execution_price: Option<usize>,
    /// the price the position was actually closed at
    pub exit_price: Option<usize>,
    /// the time the position was actually closed
    pub exit_time: Option<u64>,
}

impl Position {
    /// Returns the price the position would execute at if the prices are at
    /// levels such that the position can open, else returns None.
    pub fn is_open_satisfied(&self, bid: usize, ask: usize) -> Option<usize> {
        // only meant to be used for pending positions
        assert_eq!(self.execution_price, None);
        // only meant for limit/entry orders
        assert!(self.price.is_some());

        if self.long {
            if ask <= self.price.unwrap() {
                return Some(ask)
            };
        } else {
            if bid >= self.price.unwrap() {
                return Some(bid)
            };
        };

        None
    }

    /// Returns the price the position would execute at if the position meets
    /// the conditions for closure and the reason for its closure, else returns None.
    pub fn is_close_satisfied(&self, bid: usize, ask: usize) -> Option<(usize, PositionClosureReason)> {
        // only meant to be used for open positions
        assert!(self.execution_price.is_some());
        assert_eq!(self.exit_price, None);

        if self.long {
            if self.stop.is_some() && self.stop.unwrap() >= bid {
                return Some( (bid, PositionClosureReason::StopLoss) );
            } else if self.take_profit.is_some() && self.take_profit.unwrap() <= ask {
                return Some( (ask, PositionClosureReason::StopLoss) );
            }
        } else {
            if self.stop.is_some() && self.stop.unwrap() <= ask {
                return Some( (ask, PositionClosureReason::TakeProfit) )
            } else if self.take_profit.is_some() && self.take_profit.unwrap() >= bid {
                return Some( (bid, PositionClosureReason::TakeProfit) );
            }
        }

        None
    }
}

/// Returns a struct given the struct's field:value pairs in a HashMap.  If
/// the provided HashMap doesn't contain a field, then the default is used.
pub trait FromHashmap<T> : Default {
    fn from_hashmap(hm: HashMap<String, String>) -> T;
}

/// Settings for the simulated broker that determine things like trade fees,
/// estimated slippage, etc.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
// procedural macro is defined in the `from_hashmap` crate found in the util
// directory's root.
#[derive(FromHashmap)]
pub struct SimBrokerSettings {
    pub starting_balance: f64,
    /// how many microseconds ahead the broker is to the client.
    pub ping_ns: usize,
    /// how many us between when the broker receives an order and executes it.
    pub execution_delay_us: usize,
    /// the minimum size that a trade can be in cents of .
    pub lot_size: usize,
    pub leverage: usize,
}

impl Default for SimBrokerSettings {
    fn default() -> SimBrokerSettings {
        SimBrokerSettings {
            starting_balance: 1f64,
            ping_ns: 0,
            execution_delay_us: 0usize,
            lot_size: 1000,
            leverage: 50,
        }
    }
}

#[test]
fn simbroker_settings_hashmap_population() {
    let mut hm = HashMap::new();
    hm.insert(String::from("ping_ns"), String::from("2000"));
    let settings = SimBrokerSettings::from_hashmap(hm);
    assert_eq!(settings.ping_ns, 2000);
}
