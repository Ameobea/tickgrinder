//! Simulated broker used for backtests.  Contains facilities for simulating trades,
//! managing balances, and reporting on statistics from previous trades.

use std::collections::HashMap;
use std::sync::atomic::{Ordering, AtomicUsize};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
#[allow(unused_imports)]
use test;

use futures::Future;
use futures::{oneshot, Oneshot};
use futures::stream::{channel, Stream, Receiver};
use uuid::Uuid;

use algobot_util::trading::tick::*;
use algobot_util::trading::broker::*;

// TODO: Wire TickSink into SimBroker so that the broker always receives up-to-date data

/// Settings for the simulated broker that determine things like trade fees,
/// estimated slippage, etc.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SimBrokerSettings {
    pub starting_balance: f64,
    pub ping_ms: f64 // how many ms ahead the broker is to the client
}

impl SimBrokerSettings {
    /// Creates a default SimBrokerSettings used for tests
    pub fn default() -> SimBrokerSettings {
        SimBrokerSettings {
            starting_balance: 1f64,
            ping_ms: 0f64,
        }
    }

    /// Parses a String:String hashmap into a SimBrokerSettings object.
    pub fn from_hashmap(hm: HashMap<String, String>) -> Result<SimBrokerSettings, BrokerError> {
        let mut settings = SimBrokerSettings::default();

        for (k, v) in hm.iter() {
            match k.as_str() {
                "starting_balance" => {
                    settings.starting_balance = v.parse::<f64>().unwrap_or({
                        return Err(SimBrokerSettings::kv_parse_error(k, v))
                    });
                },
                "ping_ms" => {
                    settings.ping_ms = v.parse::<f64>().unwrap_or({
                        return Err(SimBrokerSettings::kv_parse_error(k, v))
                    });
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

/// A simulated broker that is used as the endpoint for trading activity in backtests.
pub struct SimBroker {
    pub accounts: Arc<Mutex<HashMap<Uuid, Account>>>,
    pub settings: SimBrokerSettings,
    tick_receivers: HashMap<String, Receiver<Tick, ()>>,
    prices: HashMap<String, Arc<(AtomicUsize, AtomicUsize)>>, // broker's view of prices in pips
    push_stream_handle: mpsc::SyncSender<Result<BrokerMessage, BrokerError>>,
    push_stream_recv: Option<Receiver<BrokerMessage, BrokerError>>,
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
        let (mpsc_s, f_r) = SimBroker::init_stream();

        SimBroker {
            accounts: Arc::new(Mutex::new(accounts)),
            settings: settings,
            tick_receivers: HashMap::new(),
            prices: HashMap::new(),
            push_stream_handle: mpsc_s,
            push_stream_recv: Some(f_r),
        }
    }
}

impl Broker for SimBroker {
    fn init(&mut self, settings: HashMap<String, String>) -> Oneshot<Result<Self, BrokerError>> {
        let (c, o) = oneshot::<Result<Self, BrokerError>>();
        let broker_settings = SimBrokerSettings::from_hashmap(settings);
        if broker_settings.is_ok() {
            c.complete(Ok(SimBroker::new(broker_settings.unwrap())));
        } else {
            c.complete(Err(broker_settings.unwrap_err()));
        }

        o
    }

    fn get_ledger(&mut self, account_id: Uuid) -> Oneshot<Result<Ledger, BrokerError>> {
        let (complete, oneshot) = oneshot::<Result<Ledger, BrokerError>>();
        let account = self.get_ledger_clone(account_id);
        complete.complete(account);

        oneshot
    }

    fn list_accounts(&mut self) -> Oneshot<Result<HashMap<Uuid, Account>, BrokerError>> {
        let (complete, oneshot) = oneshot::<Result<HashMap<Uuid, Account>, BrokerError>>();
        let accounts;
        {
            let _accounts = self.accounts.lock().unwrap();
            accounts = _accounts.clone();

        }
        complete.complete(Ok(accounts));
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

    #[allow(unreachable_code)]
    fn get_stream(&mut self) -> Result<Receiver<BrokerMessage, BrokerError>, BrokerError> {
        if self.push_stream_recv.is_none() {
            return Err(BrokerError::Message{
                message: "You already took a handle to the push stream and can't take another.".to_string()
            })
        }

        Ok(self.push_stream_recv.take().unwrap())
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
        let accounts_clone = self.accounts.clone();
        let push_handle = self.get_push_handle();
        let wired_tickstream = wire_tickstream(
            price_arc, symbol, raw_tickstream, accounts_clone, push_handle
        );
        Ok(wired_tickstream)
    }
}

impl SimBroker {
    /// Sends a message over the broker's push channel
    pub fn push_msg(&self, msg: BrokerResult) {
        let ref sender = self.push_stream_handle;
        sender.send(msg).unwrap_or({/* Sender disconnected, shutting down. */});
    }

    /// Returns a handle with which to send push messages
    pub fn get_push_handle(&self) -> mpsc::SyncSender<Result<BrokerMessage, BrokerError>> {
        self.push_stream_handle.clone()
    }

    /// Initializes the push stream by creating internal messengers
    #[allow(unreachable_code)] // necessary because bug
    fn init_stream() -> (mpsc::SyncSender<Result<BrokerMessage, BrokerError>>, Receiver<BrokerMessage, BrokerError>) {
        let (mpsc_s, mpsc_r) = mpsc::sync_channel::<Result<BrokerMessage, BrokerError>>(5);
        let (mut f_s, f_r) = channel::<BrokerMessage, BrokerError>();

        thread::spawn(move || {
            // block until message received over a mpsc sender
            // then re-transmit them through the push stream
            for message in mpsc_r.iter() {
                match message {
                    Ok(message) => {
                        f_s = f_s.send(Ok(message)).wait().ok().unwrap();
                    },
                    Err(err_msg) => {
                        f_s = f_s.send(Err(err_msg)).wait().ok().unwrap();
                    },
                }
            }
        });

        (mpsc_s, f_r)
    }

    /// actually executes an action sent to the SimBroker
    pub fn exec_action(&mut self, cmd: &BrokerAction) -> BrokerResult {
        match cmd {
            &BrokerAction::MarketOrder{
                account, ref symbol, long, size, stop, take_profit, max_range
            } => {
                self.market_open(account, symbol, long, size, stop, take_profit, max_range)
            },
            _ => Err(BrokerError::Unimplemented{
                message: "SimBroker doesn't support that action.".to_string()
            })
        }
    }

    /// Opens a position at the current market price with options for settings stop
    /// loss, take profit.
    fn market_open(
        &mut self, account: Uuid, symbol: &String, long: bool, size: usize,stop: Option<usize>,
        take_profit: Option<usize>, max_range: Option<f64>
    ) -> BrokerResult {
        unimplemented!();
    }

    /// Dumps the SimBroker state to a file that can be resumed later.
    pub fn dump_to_file(&mut self, filename: &str) {
        unimplemented!();
    }

    /// Returns a clone of an account or an error if it doesn't exist.
    pub fn get_ledger_clone(&mut self, uuid: Uuid) -> Result<Ledger, BrokerError> {
        let accounts = self.accounts.lock().unwrap();
        match accounts.get(&uuid) {
            Some(acct) => Ok(acct.ledger.clone()),
            None => Err(BrokerError::Message{
                message: "No account exists with that UUID.".to_string()
            }),
        }
    }

    /// Called each tick to check if any pending positions need opening or closing.
    pub fn tick_positions(
        symbol: String,
        sender_handle: &mpsc::SyncSender<Result<BrokerMessage, BrokerError>>,
        accounts_mutex: Arc<Mutex<HashMap<Uuid, Account>>>,
        price_arc: Arc<(AtomicUsize, AtomicUsize)>,
        timestamp: u64
    ) {
        let mut accounts = accounts_mutex.lock().unwrap();
        for (acct_id, mut acct) in accounts.iter_mut() {
            let (ref bid_atom, ref ask_atom) = *price_arc;
            let (bid, ask) = (bid_atom.load(Ordering::Relaxed), ask_atom.load(Ordering::Relaxed));
            let mut satisfied_pendings = Vec::new();

            for (pos_id, pos) in acct.ledger.pending_positions.iter() {
                let satisfied = pos.is_open_satisfied(bid, ask);
                // market conditions have changed and this position should be opened
                if pos.symbol == symbol && satisfied.is_some() {
                    satisfied_pendings.push( (pos_id.clone(), satisfied) );
                }
            }

            // fill all the satisfied pending positions
            for (pos_id, price_opt) in satisfied_pendings {
                let mut pos = acct.ledger.pending_positions.remove(&pos_id).unwrap();
                pos.execution_time = Some(timestamp);
                pos.execution_price = price_opt;
                // TODO: Adjust account balance and stats
                acct.ledger.open_positions.insert(pos_id, pos.clone());
                // send push message with notification of fill
                let _ = sender_handle.send(
                    Ok(BrokerMessage::PositionOpened{
                        position_id: pos_id, position: pos, timestamp: timestamp
                    })
                );
            }

            let mut satisfied_opens = Vec::new();
            for (pos_id, pos) in acct.ledger.open_positions.iter() {
                let satisfied = pos.is_close_satisfied(bid, ask);
                // market conditions have changed and this position should be closed
                if pos.symbol == symbol && satisfied.is_some() {
                    satisfied_opens.push( (pos_id.clone(), satisfied) );
                }
            }

            // close all the satisfied open positions
            for (pos_id, closure) in satisfied_opens {
                let (close_price, closure_reason) = closure.unwrap();
                let mut pos = acct.ledger.pending_positions.remove(&pos_id).unwrap();
                pos.exit_time = Some(timestamp);
                pos.exit_price = Some(close_price);
                // TODO: Adjust account balance and stats
                acct.ledger.closed_positions.insert(pos_id, pos.clone());
                // send push message with notification of close
                let _ = sender_handle.send(
                    Ok(BrokerMessage::PositionClosed{
                        position_id: pos_id, position: pos, reason: closure_reason, timestamp: timestamp
                    })
                );
            }
        }
    }

    /// Returns the current price for a given symbol or None if the SimBroker
    /// doensn't have a price.
    pub fn get_price(&self, symbol: String) -> Option<u64> {
        let opt = self.prices.get(&symbol).unwrap_or({
            return None
        });

    }
}

/// Called during broker initialization.  Takes a stream of live ticks from the backtester
/// and uses it to power its own prices, returning a Stream that can be passed off to
/// a client to serve as its price feed.
fn wire_tickstream(
    price_arc: Arc<(AtomicUsize, AtomicUsize)>, symbol: String, tickstream: Receiver<Tick, ()>,
    accounts: Arc<Mutex<HashMap<Uuid, Account>>>,
    push_stream_handle: mpsc::SyncSender<Result<BrokerMessage, BrokerError>>
) -> Box<Stream<Item=Tick, Error=()>> {
    Box::new(tickstream.map(move |t| {
        let (ref bid_atom, ref ask_atom) = *price_arc;

        // convert the tick's prices to pips and store
        bid_atom.store(Tick::price_to_pips(t.bid), Ordering::Relaxed);
        ask_atom.store(Tick::price_to_pips(t.ask), Ordering::Relaxed);

        // check if any positions need to be opened/closed due to this tick
        SimBroker::tick_positions(symbol.clone(), &push_stream_handle, accounts.clone(), price_arc.clone(), t.timestamp as u64);
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

/// How long it takes to unwrap the mpsc sender, send a message, and re-store the sender.
#[bench]
fn send_push_message(b: &mut test::Bencher) {
    let mut sim_b = SimBroker::new(SimBrokerSettings::default());
    let receiver = sim_b.get_stream();
    assert!(receiver.is_ok());
    receiver.unwrap().for_each(|msg| {
        // println!("{:?}", msg );
        Ok(())
    }).forget();

    b.iter(|| {
        sim_b.push_msg(Ok(BrokerMessage::Success))
    })
}
