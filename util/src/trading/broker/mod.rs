//! Represents a broker - the endpoint for all trading activity on the platform.
//! Also contains helper functions for managing accounts.

use std::collections::HashMap;

use uuid::Uuid;
use futures::sync::oneshot::Receiver;
use futures::stream::Stream;
use futures::sync::mpsc::UnboundedReceiver;

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
    fn get_stream(&mut self) -> Result<UnboundedReceiver<BrokerResult>, BrokerError>;

    /// Returns a stream of live ticks for a symbol.
    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()> + Send>, BrokerError>;
}

/// Utility type for a broker response that may fail
pub type BrokerResult = Result<BrokerMessage, BrokerError>;

/// Utility type for a currently pending broker action
pub type PendingResult = Receiver<BrokerResult>;

// TODO: Move SimbrokerSettings out of here

/// Settings for the simulated broker that determine things like trade fees,
/// estimated slippage, etc.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
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

impl SimBrokerSettings {
    /// Creates a default SimBrokerSettings used for tests
    pub fn default() -> SimBrokerSettings {
        SimBrokerSettings {
            starting_balance: 1f64,
            ping_ns: 0,
            execution_delay_us: 0usize,
            lot_size: 100000,
            leverage: 50,
        }
    }

    /// Parses a String:String hashmap into a SimBrokerSettings object.
    pub fn from_hashmap(hm: HashMap<String, String>) -> Result<SimBrokerSettings, BrokerError> {
        let mut settings = SimBrokerSettings::default();

        // TODO: change to use Configurator settings.

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
                    let res = v.parse::<usize>();
                    if res.is_err() {
                        return Err(SimBrokerSettings::kv_parse_error(k, v))
                    };
                    settings.ping_ns = res.unwrap();
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
