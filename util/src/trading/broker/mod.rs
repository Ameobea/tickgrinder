//! Represents a broker - the endpoint for all trading activity on the platform.
//! Also contains helper functions for managing accounts.

use std::collections::HashMap;

use uuid::Uuid;
use futures::sync::oneshot::Receiver;
use futures::stream::Stream;
use futures::sync::mpsc::UnboundedReceiver;

use trading::tick::Tick;
use trading::trading_condition::*;

/// A broker is the endpoint for all trading actions taken by the platform.  It processes
/// trades and supplies information about the condition of portfolios.  The Broker trait
/// acts as a wrapper for individual broker APIs.
pub trait Broker {
    /// Creates a connection to the broker and initializes its internal environment.
    /// Takes a Key:Value HashMap containing configuration settings.
    fn init(&mut self, settings: HashMap<String, String>) -> Receiver<Result<Self, BrokerError>> where Self:Sized;

    /// Returns a list of all accounts the user has on the broker.
    fn list_accounts(&mut self) -> Receiver<Result<HashMap<Uuid, Account>, BrokerError>>;

    /// Returns a Ledger containing the Broker's version of all current and closed
    /// trades and positions as well as balance and portfolio state.
    fn get_ledger(&mut self, account_id: Uuid) -> Receiver<Result<Ledger, BrokerError>>;

    /// Executes a BrokerAction on the broker, returning its response.
    fn execute(&mut self, action: BrokerAction) -> PendingResult;

    /// Returns a stream of messages pushed from the broker that do not originate from an
    /// action sent to the broker.  These can be things like notifications of closed positions,
    /// orders being filled, etc.
    fn get_stream(&mut self) -> Result<UnboundedReceiver<BrokerResult>, BrokerError>;

    /// Returns a stream of live ticks for a symbol.
    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()> + Send>, BrokerError>;
}

/// Utility type for a broker response that may fail
pub type BrokerResult = Result<BrokerMessage, BrokerError>;

/// Utility type for a currently pending broker action
pub type PendingResult = Receiver<BrokerResult>;

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
    InsufficientBalance,
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
    pub fn open_position(&mut self, pos: Position) -> BrokerResult {
        let uuid = Uuid::new_v4();
        let execution_time = pos.execution_time.unwrap();
        if pos.price.is_none() {
            return Err(BrokerError::Message{
                message: "The supplied position does not have an entry price.".to_string()
            })
        }

        let cost = (pos.price.unwrap() * pos.size as usize) as f64;
        if cost > self.balance as f64 {
            return Err(BrokerError::InsufficientBalance)
        }
        self.balance -= cost;

        self.open_positions.insert(uuid, pos.clone());
        Ok(BrokerMessage::PositionOpened{
            position_id: uuid,
            position: pos,
            timestamp: execution_time,
        })
    }

    /// Closes the position with the specified Uuid.  Returns an error if no position with that Uuid
    /// exists in the ledger.
    pub fn close_position(&mut self, uuid: Uuid) -> BrokerResult {
        let res = self.open_positions.remove(&uuid);
        if res.is_none() {
            return Err(BrokerError::NoSuchPosition)
        }
        Ok(BrokerMessage::Success)
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

    /// Returns the price the position would execute at if the position meets the condition for closure
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

/// Settings for the simulated broker that determine things like trade fees,
/// estimated slippage, etc.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SimBrokerSettings {
    pub starting_balance: f64,
    // how many ms ahead the broker is to the client.
    pub ping_ms: f64,
    // how many us between when the broker receives an order and executes it.
    pub execution_delay_us: usize,
}

impl SimBrokerSettings {
    /// Creates a default SimBrokerSettings used for tests
    pub fn default() -> SimBrokerSettings {
        SimBrokerSettings {
            starting_balance: 1f64,
            ping_ms: 0f64,
            execution_delay_us: 0usize,
        }
    }

    /// Parses a String:String hashmap into a SimBrokerSettings object.
    pub fn from_hashmap(hm: HashMap<String, String>) -> Result<SimBrokerSettings, BrokerError> {
        let mut settings = SimBrokerSettings::default();

        for (k, v) in hm.iter() {
            match k.as_str() {
                "starting_balance" => {
                    let res = v.parse::<f64>();
                    if res.is_err() {
                        return Err(SimBrokerSettings::kv_parse_error(k, v))
                    }
                    settings.starting_balance = res.unwrap();
                },
                "ping_ms" => {
                    let res = v.parse::<f64>();
                    if res.is_err() {
                        return Err(SimBrokerSettings::kv_parse_error(k, v))
                    };
                    settings.ping_ms = res.unwrap();
                },
                _ => (),
            }
        }

        Ok(settings)
    }

    fn kv_parse_error(k: &String, v: &String) -> BrokerError {
        return BrokerError::Message{
            message: format!("Unable to parse K:V pair: {}:{}", k, v)
        }
    }
}
