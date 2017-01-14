//! Holds definitions of the internal representations of trading objects and
//! abstractions for messages sent and received to brokers.

use std::collections::HashMap;

use uuid::Uuid;

use trading::trading_condition::{TradingAction};
use trading::broker::*;

/// An account
#[derive(Clone, Debug)]
pub struct Account {
    pub uuid: Uuid,
    pub ledger: Ledger,
    pub live: bool, // false if a demo account
}

/// Any action that the platform can take using the broker
#[derive(Clone, Debug, PartialEq)]
pub enum BrokerAction {
    TradingAction{ action: TradingAction },
    /// Returns a Pong with the timestamp the broker received the message
    Ping,
    Disconnect,
}

/// A response from a broker indicating the result of an action.
#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrokerError {
    Message{message: String},
    Unimplemented{message: String}, // the broker under the wrapper can't do what you asked it
    InsufficientBuyingPower,
    NoSuchPosition,
    NoSuchAccount,
    NoSuchSymbol,
    InvalidModificationAmount,
    NoDataAvailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PositionClosureReason {
    StopLoss,
    TakeProfit,
    MarginCall,
    Expired,
    FillOrKill,
    MarketClose,
}

/// The platform's internal representation of the current state of an account.
/// Contains information about past trades as well as current positions.
#[derive(Clone, Debug)]
pub struct Ledger {
    pub balance: usize,
    pub pending_positions: HashMap<Uuid, Position>,
    pub open_positions: HashMap<Uuid, Position>,
    pub closed_positions: HashMap<Uuid, Position>,
}

impl Ledger {
    pub fn new(starting_balance: usize) -> Ledger {
        Ledger {
            balance: starting_balance,
            pending_positions: HashMap::new(),
            open_positions: HashMap::new(),
            closed_positions: HashMap::new(),
        }
    }

    /// Opens the supplied position in the ledger.  Returns an error if there are insufficient funds
    /// in the ledger to open the position in units of the base currency.
    ///
    /// `margin_requirement` is the dollar value of the position * leverage.
    pub fn open_position(&mut self, pos: Position, margin_requirement: usize) -> BrokerResult {
        let uuid = Uuid::new_v4();
        // we assume that the supplied execution time is valid here
        let execution_time = pos.execution_time.unwrap();
        if pos.price.is_none() {
            return Err(BrokerError::Message{
                message: "The supplied position does not have an entry price.".to_string()
            })
        }

        if margin_requirement > self.balance {
            return Err(BrokerError::InsufficientBuyingPower)
        }
        self.balance -= margin_requirement;

        self.open_positions.insert(uuid, pos.clone());
        Ok(BrokerMessage::PositionOpened{
            position_id: uuid,
            position: pos,
            timestamp: execution_time,
        })
    }

    /// Completely closes the specified condition at the given price, crediting the account the
    /// funds yielded.  Timestamp is the time the order was submitted + any simulated delays.
    pub fn close_position(&mut self, uuid: Uuid, position_value: usize, timestamp: u64) -> BrokerResult {
        let pos = self.open_positions.remove(&uuid);
        if pos.is_none() {
            return Err(BrokerError::NoSuchPosition)
        }
        self.balance += position_value;

        Ok(BrokerMessage::PositionClosed{
            position: pos.unwrap(),
            position_id: uuid,
            reason: PositionClosureReason::MarketClose,
            timestamp: timestamp,
        })
    }

    /// Increases or decreases the size of the specified position by the given amount.  Returns errors
    /// if the account doesn't have enough buying power to execute the action or if a position with
    /// the specified UUID doesn't exist.
    pub fn modify_position(&mut self, uuid: Uuid, units: isize, modification_cost: usize, timestamp: u64) -> BrokerResult {
        let mut pos = match self.open_positions.remove(&uuid) {
            Some(p) => p,
            None => {
                return Err(BrokerError::NoSuchPosition);
            },
        };

        let unit_diff = units + (pos.size as isize);
        if unit_diff < 0 {
            return Err(BrokerError::InvalidModificationAmount);
        } else if unit_diff == 0 {
            return self.close_position(uuid, modification_cost, timestamp);
        }

        if self.balance < modification_cost {
            return Err(BrokerError::InsufficientBuyingPower);
        }

        // everything seems to be in order, so do the modification
        pos.size = ((pos.size as isize) + units) as usize;
        self.balance -= modification_cost;
        self.open_positions.insert(uuid, pos.clone());

        Ok(BrokerMessage::PositionModified{
            position: pos,
            position_id: uuid,
            timestamp: timestamp,
        })
    }
}

/// Represents an opened, closed, or pending position on a broker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub creation_time: u64,
    pub symbol: String,
    pub size: usize,
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

/// Returns a struct given the struct's field:value pairs in a HashMap.  If the provided HashMap
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
    pub ping_ns: usize,
    /// How many nanoseconds between when the broker receives an order and executes it
    pub execution_delay_ns: usize,
    /// Buying power is leverage * balance
    pub leverage: usize,
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
        SimBrokerSettings {
            starting_balance: 50 * 1000 * 100, // $50,000
            ping_ns: 0,
            execution_delay_ns: 0usize,
            leverage: 50,
            fx: true,
            fx_base_currency: String::from("USD"),
            fx_lot_size: 1000,
            fx_accurate_pricing: false,
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
