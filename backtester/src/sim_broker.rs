//! Simulated broker used for backtests.  Contains facilities for simulating trades,
//! managing balances, and reporting on statistics from previous trades.

// TODO: Write about how SimBroker is 100% event-based and only accepts actions in
// response to ticks.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::atomic::{Ordering, AtomicUsize};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
#[allow(unused_imports)]
use test;

use futures::{oneshot, Oneshot};
use futures::stream::{BoxStream, Stream};
use futures::sync::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

use tickgrinder_util::trading::tick::*;
pub use tickgrinder_util::trading::broker::*;
use tickgrinder_util::trading::trading_condition::*;
use tickgrinder_util::transport::command_server::CommandServer;

/// Contains a stream that yeilds the ticks that power the SimBroker as well as some
/// metadata about the data source.
pub struct InputTickstream {
    /// The stream that actually yeilds the ticks
    pub stream: BoxStream<Tick, ()>,
    /// `true` if the ticks are an exchange rate
    /// The symbol must be six characters like "EURUSD"
    pub is_fx: bool,
    /// Decimal precision of the input ticks
    pub decimal_precision: usize,
}

/// A simulated broker that is used as the endpoint for trading activity in backtests.
pub struct SimBroker {
    /// Contains all the accounts simulated by the simbroker
    pub accounts: Arc<Mutex<HashMap<Uuid, Account>>>,
    /// A copy of the settings generated from the input HashMap
    pub settings: SimBrokerSettings,
    /// Streams that generate the ticks used to power the SimBroker
    /// They usually come from a backtest.
    pub tick_receivers: HashMap<String, InputTickstream>,
    /// Broker's view of prices in pips, determined by the `tick_receiver`s
    prices: HashMap<String, Arc<(AtomicUsize, AtomicUsize)>>,
    /// Timestamp of last price update received by broker
    timestamp: Arc<AtomicUsize>,
    /// A handle to the sender for the channel through which push messages are sent
    push_stream_handle: UnboundedSender<BrokerResult>,
    /// A handle to the receiver for the channel throgh which push messages are received
    push_stream_recv: Option<UnboundedReceiver<BrokerResult>>,
    /// The CommandServer used for logging
    pub cs: CommandServer,
}

impl SimBroker {
    pub fn new(settings: SimBrokerSettings, cs: CommandServer) -> SimBroker {
        let mut accounts = HashMap::new();
        let account = Account {
            uuid: Uuid::new_v4(),
            ledger: Ledger::new(settings.starting_balance),
            live: false,
        };
        accounts.insert(Uuid::new_v4(), account);
        let (tx, rx) = unbounded::<BrokerResult>();

        SimBroker {
            accounts: Arc::new(Mutex::new(accounts)),
            settings: settings,
            tick_receivers: HashMap::new(),
            prices: HashMap::new(),
            timestamp: Arc::new(AtomicUsize::new(0)),
            push_stream_handle: tx,
            push_stream_recv: Some(rx),
            cs: cs,
        }
    }
}

impl Broker for SimBroker {
    fn init(settings: HashMap<String, String>) -> Oneshot<Result<Self, BrokerError>> {
        let (c, o) = oneshot::<Result<Self, BrokerError>>();
        // this currently panics if you give it bad values...
        // TODO: convert FromHashmap to return a Result<SimbrokerSettings>
        let broker_settings = SimBrokerSettings::from_hashmap(settings);
        let cs = CommandServer::new(Uuid::new_v4(), "Simbroker");
        c.complete(Ok(SimBroker::new(broker_settings, cs)));

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

    #[allow(unreachable_code)]
    fn execute(&mut self, action: BrokerAction) -> PendingResult {
        let (complete, oneshot) = oneshot::<BrokerResult>();

        // TODO
        let reply: BrokerResult = match action {
            BrokerAction::Ping => {
                unimplemented!(); // TODO
            },
            BrokerAction::TradingAction{action} => {
                match action {
                    TradingAction::MarketOrder{account, symbol, long, size, stop, take_profit, max_range} => {
                        unimplemented!(); // TODO
                    },
                    TradingAction::MarketClose{uuid, size} => {
                        unimplemented!(); // TODO
                    }
                    TradingAction::LimitOrder{account, symbol, long, size, stop, take_profit, entry_price} => {
                        unimplemented!(); // TODO
                    },
                    TradingAction::LimitClose{uuid, size, exit_price} => {
                        unimplemented!(); // TODO
                    },
                    TradingAction::ModifyPosition{uuid, stop, take_profit, entry_price} => {
                        unimplemented!(); // TODO
                    }
                }
            },
        };

        oneshot
    }

    fn get_stream(&mut self) -> Result<UnboundedReceiver<BrokerResult>, BrokerError> {
        if self.push_stream_recv.is_none() {
            // TODO: Enable multiple handles to be taken?
            return Err(BrokerError::Message{
                message: "You already took a handle to the push stream and can't take another.".to_string()
            })
        }

        Ok(self.push_stream_recv.take().unwrap())
    }

    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()> + Send>, BrokerError> {
        let opt = self.tick_receivers.remove(&symbol);
        if opt.is_none() {
            return Err(BrokerError::Message{
                message: "No data source available for that symbol or \
                a stream to that symbol has already been opened.".to_string()
            })
        }
        let tickstream = opt.unwrap();
        Ok(tickstream.stream)
    }
}

impl SimBroker {
    /// Sends a message over the broker's push channel
    pub fn push_msg(&self, msg: BrokerResult) {
        let ref sender = self.push_stream_handle;
        sender.send(msg).expect("Unable to push_msg");
    }

    /// Returns a handle with which to send push messages
    pub fn get_push_handle(&self) ->UnboundedSender<Result<BrokerMessage, BrokerError>> {
        self.push_stream_handle.clone()
    }

    /// Initializes the push stream by creating internal messengers
    fn init_stream() -> (mpsc::Sender<Result<BrokerMessage, BrokerError>>, UnboundedReceiver<BrokerResult>) {
        let (mpsc_s, mpsc_r) = mpsc::channel::<Result<BrokerMessage, BrokerError>>();
        let (f_s, f_r) = unbounded::<BrokerResult>();

        thread::spawn(move || {
            // block until message received over a mpsc sender
            // then re-transmit them through the push stream
            for message in mpsc_r.iter() {
                match message {
                    Ok(message) => {
                        f_s.send(Ok(message)).unwrap();
                    },
                    Err(err_msg) => {
                        f_s.send(Err(err_msg)).unwrap();
                    },
                }
            }
            println!("After init_stream() channel conversion loop!!");
        });

        (mpsc_s, f_r)
    }

    /// actually executes an action sent to the SimBroker
    pub fn exec_action(&mut self, cmd: &BrokerAction) -> BrokerResult {
        match cmd {
            &BrokerAction::TradingAction{action: TradingAction::MarketOrder{
                account, ref symbol, long, size, stop, take_profit, max_range
            }} => {
                let timestamp = self.get_timestamp();
                assert!(timestamp != 0);
                self.market_open(account, symbol, long, size, stop, take_profit, max_range, timestamp)
            },
            _ => Err(BrokerError::Unimplemented{
                message: "SimBroker doesn't support that action.".to_string()
            })
        }
    }

    /// Opens a position at the current market price with options for settings stop
    /// loss, take profit.
    fn market_open(
        &mut self, account_id: Uuid, symbol: &String, long: bool, size: usize, stop: Option<usize>,
        take_profit: Option<usize>, max_range: Option<f64>, timestamp: u64
    ) -> BrokerResult {
        let opt = self.get_price(symbol);
        if opt.is_none() {
            return Err(BrokerError::NoSuchSymbol)
        }
        let (bid, ask) = opt.unwrap();
        let is_fx = self.tick_receivers.get(symbol).unwrap().is_fx;

        let cur_price;
        if long {
            cur_price = ask;
        } else {
            cur_price = bid;
        }

        let pos = Position {
            creation_time: timestamp,
            symbol: symbol.clone(),
            size: size,
            price: Some(cur_price),
            long: long,
            stop: stop,
            take_profit: take_profit,
            execution_time: Some(timestamp + self.settings.execution_delay_ns as u64),
            execution_price: Some(cur_price),
            exit_price: None,
            exit_time: None,
        };

        let open_cost;
        if is_fx {
            let primary_currency = &symbol[0..3];
            let base_rate = try!(self.get_base_rate(primary_currency));
            open_cost = (size * base_rate) / self.settings.leverage;
        } else {
            open_cost = size / self.settings.leverage;
        }

        let mut accounts = self.accounts.lock().unwrap();
        let account_ = accounts.entry(account_id);
        match account_ {
            Entry::Occupied(mut occ) => {
                let mut account = occ.get_mut();
                return account.ledger.open_position(pos, open_cost);
            },
            Entry::Vacant(_) => {
                return Err(BrokerError::NoSuchAccount);
            }
        }
    }

    /// Dumps the SimBroker state to a file that can be resumed later.
    pub fn dump_to_file(&mut self, filename: &str) {
        unimplemented!(); // TODO
    }

    /// Used for Forex exchange rate conversions.  The cost to open a position is determined
    /// by the exchange rate between the base currency and the primary currency of the pair.
    ///
    /// Gets the conversion rate (in pips) between the base currency of the simbroker and
    /// the supplied currency.  If the base currency is USD and AUD is provided, the exchange
    /// rate for AUD/USD will be returned.  Returns Err if we lack the data to do that.
    pub fn get_base_rate(&self, symbol: &str) -> Result<usize, BrokerError> {
        if !self.settings.fx {
            return Err(BrokerError::Message{
                message: String::from("Can only convert to base rate when in FX mode.")
            });
        }

        let ref base_currency = self.settings.fx_base_currency;
        let base_pair = format!("{}{}", symbol, base_currency);

        let (bid, ask) = match self.get_price(&base_pair) {
            Some(price) => price,
            None => {
                return Err(BrokerError::NoDataAvailable);
            },
        };

        Ok(ask)
    }

    /// Returns the worth of a position in units of base currency.
    pub fn get_position_value(&self, pos: &Position) -> Result<usize, BrokerError> {
        unimplemented!(); // TODO
    }

    /// Sets the price for a symbol.  If there is no price currently set for that symbol,
    /// a new entry will be added to the `self.prices` and `self.tick_receivers` HashMaps
    /// if one does not aready exist.
    pub fn oneshot_price_set(
        &mut self, symbol: String, price: (usize, usize), is_fx: bool, decimal_precision: usize,
    ) {
        let (bid, ask) = price;
        if is_fx {
            assert_eq!(symbol.len(), 6);
        }

        // insert new entry into `self.prices` or update if one exists
        match self.prices.entry(symbol.clone()) {
            Entry::Occupied(o) => {
                let (ref bid_atom, ref ask_atom) = **o.into_mut();
                bid_atom.store(bid, Ordering::Relaxed);
                ask_atom.store(ask, Ordering::Relaxed);
            },
            Entry::Vacant(v) => {
                let bid_atom = AtomicUsize::new(bid);
                let ask_atom = AtomicUsize::new(ask);
                let atom_tuple = (bid_atom, ask_atom);

                v.insert(Arc::new(atom_tuple));
            }
        }

        // insert new entry into `self.tick_receivers` if one doesn't exist
        match self.tick_receivers.entry(symbol) {
            Entry::Occupied(o) => (),
            Entry::Vacant(v) => {
                let (dummy_tx, dummy_rx) = unbounded();
                let ts = InputTickstream {
                    stream: dummy_rx.boxed(),
                    is_fx: is_fx,
                    decimal_precision: decimal_precision,
                };
                v.insert(ts);
            }
        }
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
        sender_handle: &UnboundedSender<Result<BrokerMessage, BrokerError>>,
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

    /// Registers a data source into the SimBroker.  Ticks from the supplied generator will be
    /// used to upate the SimBroker's internal prices and transmitted to connected clients.
    pub fn register_tickstream(
        &mut self, symbol: String, raw_tickstream: UnboundedReceiver<Tick>, is_fx: bool, decimal_precision: usize
    ) -> Result<(), String> {
        // wire the tickstream so that the broker updates its own prices before sending the
        // price updates off to the client
        let price_arc = self.prices.entry(symbol.clone()).or_insert(
            Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)))
        ).clone();

        // wire the tickstream so that the broker updates its own prices before sending the
        // price updates off to the client
        let accounts_clone = self.accounts.clone();
        let push_handle = self.get_push_handle();
        let timestamp_atom = self.timestamp.clone();
        let wired_tickstream = wire_tickstream(
            is_fx, decimal_precision, price_arc, symbol.clone(), raw_tickstream, accounts_clone, timestamp_atom, push_handle
        );
        self.tick_receivers.insert(symbol, wired_tickstream);
        Ok(())
    }

    /// Returns the current price for a given symbol or None if the SimBroker
    /// doensn't have a price.
    pub fn get_price(&self, symbol: &String) -> Option<(usize, usize)> {
        let opt = self.prices.get(symbol);
        if opt.is_none() {
            return None
        }
        let (ref atom_bid, ref atom_ask) = **opt.unwrap();
        Some((atom_bid.load(Ordering::Relaxed), atom_ask.load(Ordering::Relaxed)))
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp.load(Ordering::Relaxed) as u64
    }
}

/// Called during broker initialization.  Takes a stream of live ticks from the backtester
/// and uses it to power its own prices, returning a Stream that can be passed off to
/// a client to serve as its price feed.
fn wire_tickstream(
    is_fx: bool, decimal_precision: usize,
    price_arc: Arc<(AtomicUsize, AtomicUsize)>, symbol: String, tickstream: UnboundedReceiver<Tick>,
    accounts: Arc<Mutex<HashMap<Uuid, Account>>>, timestamp_atom: Arc<AtomicUsize>,
    push_stream_handle: UnboundedSender<Result<BrokerMessage, BrokerError>>
) -> InputTickstream {
    let wired_stream = tickstream.map(move |t| {
        let (ref bid_atom, ref ask_atom) = *price_arc;

        // convert the tick's prices to pips and store
        bid_atom.store(t.bid, Ordering::Relaxed);
        ask_atom.store(t.ask, Ordering::Relaxed);
        // store timestamp
        (*timestamp_atom).store(t.timestamp as usize, Ordering::Relaxed);

        // check if any positions need to be opened/closed due to this tick
        SimBroker::tick_positions(symbol.clone(), &push_stream_handle, accounts.clone(), price_arc.clone(), t.timestamp as u64);
        t
    }).boxed();

    InputTickstream {
        stream: wired_stream,
        is_fx: is_fx,
        decimal_precision: decimal_precision,
    }
}

/// It should be an error to try to subscribe to a symbol that the SimBroker doesn't keep track of.
#[test]
fn sub_ticks_err() {
    let settings = SimBrokerSettings::default();

    let mut sim_b = SimBroker::new(settings, CommandServer::new(Uuid::new_v4(), "SimBroker Test"));
    let stream = sim_b.sub_ticks("TEST".to_string());
    assert!(stream.is_err());
}

/// How long it takes to unwrap the sender, send a message, and re-store the sender.
#[bench]
fn send_push_message(b: &mut test::Bencher) {
    let settings = SimBrokerSettings::default();
    let mut sim_b = SimBroker::new(settings, CommandServer::new(Uuid::new_v4(), "SimBroker Test"));
    let receiver = sim_b.get_stream().unwrap();
    thread::spawn(move ||{
        for _ in receiver.wait() {

        }
    });

    b.iter(|| {
        sim_b.push_msg(Ok(BrokerMessage::Success))
    })
}

/// Ticks sent to the SimBroker should be re-broadcast to the client.
#[test]
fn tick_retransmission() {
    use std::sync::mpsc;

    use futures::Future;

    use data::random_reader::RandomReader;
    use data::TickGenerator;
    use backtest::{FastMap, BacktestCommand};

    // create the SimBroker
    let symbol = "TEST".to_string();
    let settings = SimBrokerSettings::default();
    let mut sim_b = SimBroker::new(settings, CommandServer::new(Uuid::new_v4(), "SimBroker Test"));
    let msg_stream = sim_b.get_stream();

    // create a random tickstream and register it to the SimBroker
    let mut gen = RandomReader::new(symbol.clone());
    let map = Box::new(FastMap {delay_ms: 1});
    let (tx, rx) = mpsc::sync_channel(5);
    let tick_stream = gen.get(map, rx);
    let res = sim_b.register_tickstream(symbol.clone(), tick_stream.unwrap(), false, 0);
    assert!(res.is_ok());

    // subscribe to ticks from the SimBroker for the test pair
    let subbed_ticks = sim_b.sub_ticks(symbol).unwrap();
    let (c, o) = oneshot::<Vec<Tick>>();
    thread::spawn(move || {
        let res = subbed_ticks
            .wait()
            .take(10)
            .map(|t| t.unwrap() )
            .collect();
        // signal once we've received all the ticks
        c.complete(res);
    });

    // start the random tick generator
    let _ = tx.send(BacktestCommand::Resume);
    // block until we've received all awaited ticks
    let res = o.wait().unwrap();
    assert_eq!(res.len(), 10);
}

#[test]
fn position_opening_closing_modification() {
    use futures::Future;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut sim = SimBroker::init(HashMap::new()).wait().unwrap().unwrap();

    let price = (0999, 1001);
    sim.oneshot_price_set(String::from("TEST"), price, false, 4);
}

#[test]
fn dynamic_base_rate_conversion() {
    use std::default::Default;
    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut settings = SimBrokerSettings::default();
    settings.fx_accurate_pricing = true;

    let mut sim = SimBroker::new(settings, cs);
}

#[test]
fn oneshot_price_setting() {
    use futures::Future;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut sim = SimBroker::init(HashMap::new()).wait().unwrap().unwrap();

    let price = (0999, 1001);
    let sym = String::from("TEST");
    sim.oneshot_price_set(sym.clone(), price, false, 4);
    assert_eq!(price, sim.get_price(&sym).unwrap());
}

#[test]
fn oneshot_base_rate_conversion() {
    use futures::Future;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut sim = SimBroker::init(HashMap::new()).wait().unwrap().unwrap();
}
