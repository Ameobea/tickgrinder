//! Simulated broker used for backtests.  Contains facilities for simulating trades,
//! managing balances, and reporting on statistics from previous trades.

use std::collections::HashMap;
use std::sync::atomic::{Ordering, AtomicUsize};
use std::sync::Arc;

use futures::{oneshot, Oneshot};
use futures::stream::{Stream, Receiver};
use uuid::Uuid;

use algobot_util::trading::tick::*;
use algobot_util::trading::broker::*;

// TODO: Wire TickSink into SimBroker so that the broker always receives up-to-date data

/// Settings for the simulated broker that determine things like trade fees,
/// estimated slippage, etc.
#[derive(Clone, Serialize, Deserialize)]
pub struct SimBrokerSettings {
    pub starting_balance: f64,
    pub ping_ms: f64 // how many ms ahead the broker is to the client
}

/// A simulated broker that is used as the endpoint for trading activity in backtests.
pub struct SimBroker {
    pub accounts: HashMap<Uuid, Account>,
    pub settings: SimBrokerSettings,
    tick_receivers: HashMap<String, Receiver<Tick, ()>>,
    prices: HashMap<String, Arc<(AtomicUsize, AtomicUsize)>>, // broker's view of prices in pips
}

impl SimBroker {
    pub fn new(settings: SimBrokerSettings) -> SimBroker {
        let mut accounts = HashMap::new();
        let account = Account {
            uuid: Uuid::new_v4(),
            ledger: Ledger::new(settings.starting_balance),
            live: false,
        };
        accounts.insert(Uuid::new_v4(), account);
        SimBroker {
            accounts: accounts,
            settings: settings,
            tick_receivers: HashMap::new(),
            prices: HashMap::new()
        }
    }
}

impl Broker for SimBroker {
    fn init(&mut self, settings: HashMap<String, String>) -> Oneshot<Self> {
        unimplemented!();
    }

    fn get_ledger(&mut self, account_id: Uuid) -> Oneshot<Result<Ledger, BrokerError>> {
        let (complete, oneshot) = oneshot::<Result<Ledger, BrokerError>>();

        let res = self.accounts.get(&account_id);
        let account = match res {
            Some(ledger) => ledger,
            None => {
                complete.complete(
                    Err(BrokerError::Message{
                        message: "No account with that UUID in this SimBroker.".to_string()
                    })
                );
                return oneshot
            }
        };
        complete.complete(Ok(account.ledger.clone()));

        oneshot
    }

    fn list_accounts(&mut self) -> Oneshot<Result<&HashMap<Uuid, Account>, BrokerError>> {
        let (complete, oneshot) = oneshot::<Result<&HashMap<Uuid, Account>, BrokerError>>();
        complete.complete(Ok(&self.accounts));
        oneshot
    }

    fn execute(&mut self, action: BrokerAction) -> PendingResult {
        let (complete, oneshot) = oneshot::<BrokerResult>();

        // TODO
        let reply: BrokerResult = match action {
            _ => Err(BrokerError::Unimplemented{message: "SimBroker doesn't support that action.".to_string()})
        };

        oneshot
    }

    fn get_stream(&mut self) -> Result<Receiver<BrokerMessage, BrokerError>, BrokerError> {
        unimplemented!();
    }

    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()>>, BrokerError> {
        let raw_tickstream = self.tick_receivers.remove(&symbol)
            .unwrap_or({
                return Err(BrokerError::Message{
                    message: "No data source available for that symbol or \
                    a stream to that symbol has already been opened.".to_string()
                })
            });

        let price_arc = self.prices.entry(symbol.clone()).or_insert(
            Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)))
        ).clone();

        // wire the tickstream so that the broker updates its own prices before sending the
        // price updates off to the client
        let wired_tickstream = wire_tickstream(price_arc, symbol, raw_tickstream);
        Ok(wired_tickstream)
    }
}

impl SimBroker {

}

/// Called during broker initialization.  Takes a stream of live ticks from the backtester
/// and uses it to power its own prices, returning a Stream that can be passed off to
/// a client to serve as its price feed.
fn wire_tickstream(
    price_arc: Arc<(AtomicUsize, AtomicUsize)>, symbol: String, tickstream: Receiver<Tick, ()>
) -> Box<Stream<Item=Tick, Error=()>> {
    Box::new(tickstream.map(move |t| {
        let (ref bid_atom, ref ask_atom) = *price_arc;

        // convert the tick's prices to pips and store
        bid_atom.store(Tick::price_to_pips(t.bid), Ordering::Relaxed);
        ask_atom.store(Tick::price_to_pips(t.ask), Ordering::Relaxed);
        t
    }))
}

#[test]
fn sub_ticks_err() {
    let settings = SimBrokerSettings{
        starting_balance: 1f64,
        ping_ms: 0f64
    };

    let mut b: SimBroker = SimBroker::new(settings);
    let stream = b.sub_ticks("TEST".to_string());
    assert!(stream.is_err());
}
