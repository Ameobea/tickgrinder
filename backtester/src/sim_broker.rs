//! Simulated broker used for backtests.  Contains facilities for simulating trades,
//! managing balances, and reporting on statistics from previous trades.

use std::collections::HashMap;

use futures::{oneshot, Oneshot};
use futures::stream::{Sender, Receiver};
use uuid::Uuid;

use algobot_util::trading::tick::*;
use algobot_util::trading::broker::*;

use DataSource;

// TODO: Wire TickSink into SimBroker so that the broker always receives up-to-date data

/// Settings for the simulated broker that determine things like trade fees,
/// estimated slippage, etc.
pub struct SimBrokerSettings {

}

/// A simulated broker that is used as the endpoint for trading activity in backtests.
pub struct SimBroker {
    accounts: Vec<Ledger>,
    settings: SimBrokerSettings,
    tick_sender: Sender<Tick, ()>,
    tick_sources: HashMap<String, DataSource>, // where to get data for each symbol of the backtest
}

impl SimBroker {
    pub fn new(settings: SimBrokerSettings) {
        unimplemented!();
    }
}

impl Broker for SimBroker {
    fn init(&mut self, settings: HashMap<String, String>) -> Oneshot<Self> {
        unimplemented!();
    }

    fn get_ledger(&mut self, account_id: Uuid) -> Oneshot<Ledger> {
        unimplemented!();
    }

    fn list_accounts(&mut self) -> Oneshot<Vec<Account>> {
        unimplemented!();
    }

    fn execute(&mut self, action: BrokerAction) -> PendingResult {
        let (complete, oneshot) = oneshot::<BrokerResult>();

        let reply: BrokerResult = match action {
            _ => Err(BrokerError::Unimplemented{message: "SimBroker doesn't support that action.".to_string()})
        };

        oneshot
    }

    fn get_stream(&mut self) -> Result<Receiver<BrokerMessage, BrokerError>, BrokerError> {
        unimplemented!();
    }

    fn sub_ticks(&mut self, symbol: String) -> Result<Receiver<Tick, ()>, BrokerError> {
        unimplemented!();
    }
}

impl SimBroker {
    /// Called every time the backtest generates a new tick in order to keep the sim broker
    /// up to date with the newest prices.
    fn tick(t: SymbolTick) {
        unimplemented!();
    }
}