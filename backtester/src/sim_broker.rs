//! Simulated broker used for backtests.  Contains facilities for simulating trades,
//! managing balances, and reporting on statistics from previous trades.

use algobot_util::trading::tick::*;
use algobot_util::trading::broker::*;

// TODO: Wire TickSink into SimBroker so that the broker always receives up-to-date data

/// Settings for the simulated broker that determine things like trade fees,
/// estimated slippage, etc.
pub struct SimBrokerSettings {

}

/// A simulated broker that is used as the endpoint for trading activity in backtests.
pub struct SimBroker {
    accounts: Vec<Ledger>
}

impl SimBroker {
    pub fn new(settings: &SimBrokerSettings) {
        unimplemented!();
    }
}

impl Broker for SimBroker {
    fn tick(t: SymbolTick) {
        unimplemented!();
    }

    fn get_ledger() -> Ledger {
        unimplemented!();
    }

    fn execute(action: BrokerAction) -> BrokerResponse {
        unimplemented!();
    }
}

