//! Represents a broker - the endpoint for all trading activity on the platform.
//! Also contains helper functions for managing accounts.

use trading::tick::SymbolTick;

/// A broker is the endpoint for all trading actions taken by the platform.  It processes
/// trades and supplies information about the condition of portfolios.
pub trait Broker {
    /// Called each time a new tick is released by the backtester
    fn tick(t: SymbolTick);

    /// Returns a Ledger containing the Broker's version of all current and closed
    /// trades and positions as well as balance and portfolio state.
    fn get_ledger() -> Ledger;

    /// Executes a BrokerAction on the broker, returning its response.
    fn execute(action: BrokerAction) -> BrokerResponse;
}

/// Any action that the platform can take using the broker
#[derive(Debug)]
pub enum BrokerAction {
    MarketBuy{symbol: String, size: usize},
    MarketStop{symbol: String, size: usize, stop: f64}
}

/// A response from a broker indicating the result of an action.
#[derive(Debug)]
pub enum BrokerResponse {
    Success, // Will be changed in the future
    Failure,
}

/// The platform's internal representation of the current state of an account.
/// Contains information about past trades as well as current positions.
pub struct Ledger {
    balance: f64,

}

impl Ledger {
    pub fn new(starting_balance: f64) {
        unimplemented!();
    }
}

/// Represents an opened, closed, or potential position on a broker.
pub struct Position {
    symbol: String,
    size: i64
}
