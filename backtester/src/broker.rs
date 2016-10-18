//! Simulated broker used for backtests.  Contains facilities for simulating trades,
//! managing balances, and reporting on statistics from previous trades.

use algobot_util::tick*;

/// A simulated broker that is used as the endpoint for trading activity in backtests.
pub struct Broker {
    accounts: Vec<Ledger>
}

// TODO: Wire TickSink into Broker so that the broker always receives up-to-date data

/// Settings for the simulated broker that determine things like trade fees,
/// estimated slippage, etc.
pub struct BrokerSettings {

}

impl Broker {
    pub fn new(settings: BrokerSettings) {
        unimplemented!();
    }

    /// Called each time a new tick is released by the backtester
    pub fn tick(t: SymbolTick) {
        unimplemented!();
    }
}

/// Any action that the platform can take using the broker
#[derive(Debug)]
pub enum BrokerAction {
    MarketBuy{symbol: String, size: usize},
    MarketStop{symbol: String, size: usize, stop: f64}
}

/// A simulated account that keeps track of open positions, historical trades, and
/// manages balances.
struct Ledger {

}

impl Ledger {
    pub fn new(starting_balance: usize) {
        unimplemented!();
    }
}
