//! Represents a broker - the endpoint for all trading activity on the platform.
//! Also contains helper functions for managing accounts.

use std::collections::HashMap;

use uuid::Uuid;
use futures::Oneshot;
use futures::stream::{Stream, Receiver};

use trading::tick::Tick;

/// A broker is the endpoint for all trading actions taken by the platform.  It processes
/// trades and supplies information about the condition of portfolios.  The Broker trait
/// acts as a wrapper for individual broker APIs.
pub trait Broker {
    /// Creates a connection to the broker and initializes its internal environment.
    /// Takes a Key:Value HashMap containing configuration settings.
    fn init(&mut self, settings: HashMap<String, String>) -> Oneshot<Self> where Self:Sized;

    /// Returns a list of all accounts the user has on the broker.
    fn list_accounts(&mut self) -> Oneshot<Result<&HashMap<Uuid, Account>, BrokerError>>;

    /// Returns a Ledger containing the Broker's version of all current and closed
    /// trades and positions as well as balance and portfolio state.
    fn get_ledger(&mut self, account_id: Uuid) -> Oneshot<Result<Ledger, BrokerError>>;

    /// Executes a BrokerAction on the broker, returning its response.
    fn execute(&mut self, action: BrokerAction) -> PendingResult;

    /// Returns a stream of messages pushed from the broker that do not originate from an
    /// action sent to the broker.  These can be things like notifications of closed positions,
    /// orders being filled, etc.
    fn get_stream(&mut self) -> Result<Receiver<BrokerMessage, BrokerError>, BrokerError>;

    /// Returns a stream of live ticks for a symbol.
    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()>>, BrokerError>;
}

/// Utility type for a broker response that may fail
pub type BrokerResult = Result<BrokerMessage, BrokerError>;

/// Utility type for a currently pending broker action
pub type PendingResult = Oneshot<BrokerResult>;

/// An account
pub struct Account {
    pub uuid: Uuid,
    pub ledger: Ledger,
    pub live: bool, // false if a demo account
}

/// Any action that the platform can take using the broker
#[derive(Debug)]
pub enum BrokerAction {
    MarketBuy{symbol: String, size: usize},
    MarketStop{symbol: String, size: usize, stop: f64}
}

/// A response from a broker indicating the result of an action.
#[derive(Debug)]
pub enum BrokerMessage {
    Success, // Will be changed in the future
    Failure,
    Notice,
}

pub enum BrokerError {
    Message{message: String},
    Unimplemented{message: String}, // the broker under the wrapper can't do what you asked it
}

/// The platform's internal representation of the current state of an account.
/// Contains information about past trades as well as current positions.
#[derive(Clone)]
pub struct Ledger {
    pub balance: f64,
    pub open_positions: Vec<Position>,
    pub closed_positions: Vec<Position>,
}

impl Ledger {
    pub fn new(starting_balance: f64) -> Ledger {
        Ledger {
            balance: starting_balance,
            open_positions: Vec::new(),
            closed_positions: Vec::new(),
        }
    }
}

/// Represents an opened, closed, or potential position on a broker.
#[derive(Clone)]
pub struct Position {
    pub symbol: String,
    pub size: i64
}
