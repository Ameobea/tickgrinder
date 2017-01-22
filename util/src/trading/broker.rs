//! Represents a broker - the endpoint for all trading activity on the platform.
//! Also contains helper functions for managing accounts.

use std::collections::HashMap;

use uuid::Uuid;
use futures::sync::oneshot::Receiver;
use futures::stream::Stream;

use trading::tick::Tick;
pub use trading::objects::*;

/// A broker is the endpoint for all trading actions taken by the platform.  It processes
/// trades and supplies information about the condition of portfolios.  The Broker trait
/// acts as a wrapper for individual broker APIs.
pub trait Broker {
    /// Creates a connection to the broker and initializes its internal environment.
    /// Takes a Key:Value HashMap containing configuration settings.
    fn init(settings: HashMap<String, String>) -> Receiver<Result<Self, BrokerError>> where Self:Sized;

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
    fn get_stream(&mut self) -> Result<Box<Stream<Item=BrokerResult, Error=()> + Send>, BrokerError>;

    /// Returns a stream of live ticks for a symbol.
    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()> + Send>, BrokerError>;
}

/// Utility type for a broker response that may fail
pub type BrokerResult = Result<BrokerMessage, BrokerError>;

/// Utility type for a currently pending broker action
pub type PendingResult = Receiver<BrokerResult>;
