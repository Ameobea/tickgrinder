//! Backtester module
//!
//! Plays back market data and executes strategies, providing a simulated broker and
//! account as well as statistics and data about the results of the strategy.

#![feature(conservative_impl_trait, associated_consts, custom_derive, proc_macro, test)]
#![allow(unused_variables, dead_code)]

extern crate algobot_util;
extern crate rand;
extern crate futures;
extern crate uuid;
extern crate redis;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate test;

mod data;
mod conf;
mod backtest;
mod sim_broker;

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use uuid::Uuid;
use futures::Future;
use futures::stream::{Stream, Sender, Receiver};

use algobot_util::transport::command_server::{CommandServer, CsSettings};
use algobot_util::transport::redis::{sub_multiple, get_client};
use algobot_util::transport::commands::*;
use algobot_util::trading::tick::Tick;
use algobot_util::trading::broker::BrokerAction;
use conf::CONF;
use backtest::*;
use data::*;
use sim_broker::*;

/// Starts the backtester module, initializing its interface to the rest of the platform
fn main() {
    let mut backtester = Backtester::new();
    backtester.listen();
}

#[derive(Clone)]
struct Backtester {
    pub uuid: Uuid,
    pub cs: CommandServer,
    pub running_backtests: Arc<Mutex<Vec<BacktestHandle>>>,
    pub simbroker_handle: Arc<Option<Sender<BrokerAction, ()>>>,
}

impl Backtester {
    pub fn new() -> Backtester {
        let settings = CsSettings {
            conn_count: 2,
            redis_host: CONF.redis_url,
            responses_channel: CONF.redis_responses_channel,
            timeout: 2020,
            max_retries: 3,
        };

        let uuid = Uuid::new_v4();

        Backtester {
            uuid: uuid,
            cs: CommandServer::new(settings),
            running_backtests: Arc::new(Mutex::new(Vec::new())),
            simbroker_handle: Arc::new(None),
        }
    }

    /// Creates a SimBroker that's managed by the Backtester.
    pub fn init_simbroker(&mut self) {
        thread::spawn(|| {

        });
    }

    /// Starts listening for commands from the rest of the platform
    pub fn listen(&mut self) {
        // subscribe to the command channels
        let rx = sub_multiple(
            CONF.redis_url, &[CONF.redis_control_channel, self.uuid.hyphenated().to_string().as_str()]
        );
        let mut redis_client = get_client(CONF.redis_url);
        let mut copy = self.clone();

        rx.for_each(move |(_, msg)| {
            let wr_cmd = match WrappedCommand::from_str(msg.as_str()) {
                Ok(wr) => wr,
                Err(e) => {
                    println!("Unable to parse WrappedCommand from String: {:?}", e);
                    return Ok(())
                }
            };

            let res: Response = match wr_cmd.cmd {
                Command::Ping => Response::Pong{ args: vec![copy.uuid.hyphenated().to_string()] },
                Command::Type => Response::Info{ info: "Backtester".to_string() },
                Command::StartBacktest{definition: definition_str} => {
                    let definition = serde_json::from_str(definition_str.as_str());
                    if definition.is_err() {
                        let err_msg = definition.err().unwrap();
                        Response::Error{
                            status: format!("Can't parse backtest defition from String: {}", err_msg)
                        }
                    } else {
                        // start the backtest and register a handle internally
                        let uuid = copy.start_backtest(definition.unwrap());

                        match uuid {
                            Ok(uuid) => Response::Info{info: uuid.hyphenated().to_string()},
                            Err(err) => Response::Error{status: err}
                        }
                    }
                },
                Command::PauseBacktest{uuid} => unimplemented!(),
                Command::ResumeBacktest{uuid} => unimplemented!(),
                Command::StopBacktest{uuid} => unimplemented!(),
                _ => Response::Error{ status: "Backtester doesn't recognize that command.".to_string() }
            };

            redis::cmd("PUBLISH")
                .arg(CONF.redis_responses_channel)
                .arg(res.wrap(wr_cmd.uuid).to_string().unwrap().as_str())
                .execute(&mut redis_client);

            Ok(())
            // TODO: Test to make sure this actually works
        }).forget();
    }

    /// Initiates a new backtest and adds it to the internal list of monitored backtests.
    fn start_backtest(
        &mut self, definition: BacktestDefinition) -> Result<Uuid, String> {
        // Create the TickGenerator that provides the backtester with data
        let mut src: Box<TickGenerator> = resolve_data_source(
            &definition.data_source, definition.symbol.clone()
        );

        // create channel for communicating messages to the running backtest sent externally
        let (external_handle_tx, handle_rx) = mpsc::sync_channel::<BacktestCommand>(5);
        // create channel for communicating messages to the running backtest internally
        let internal_handle_tx = external_handle_tx.clone();

        // modify the source tickstream to add delay between the ticks or add some other kind of
        // advanced functionality to the way they're outputted
        let tickstream: Result<Receiver<Tick, ()>, String> = match &definition.backtest_type {
            &BacktestType::Fast{delay_ms} => src.get(
                Box::new(FastMap{delay_ms: delay_ms}), handle_rx
            ),
            &BacktestType::Live => src.get(Box::new(LiveMap::new()), handle_rx),
        };

        if tickstream.is_err() {
            return Err( format!("Error creating tickstream: {}", tickstream.err().unwrap()) )
        }

        // create a TickSink that receives the output of the backtest
        let mut dst: Box<TickSink + Send> = match &definition.data_dest {
            &DataDest::Redis{ref host, ref channel} => {
                Box::new(RedisSink::new(definition.symbol.clone(), channel.clone(), host.as_str()))
            },
            &DataDest::Console => Box::new(ConsoleSink{}),
            &DataDest::Null => Box::new(NullSink{}),
        };

        let _definition = definition.clone();
        let mut i = 0;

        // initiate tick flow
        let _ = tickstream.unwrap().for_each(move |t| {
            i += 1;

            // send the tick to the sink
            dst.tick(t);

            if check_early_exit(&t, &_definition, i) {
                println!("Backtest exiting early.");
                return Err(())
            }

            Ok(())
        }).or_else(move |_| {
            println!("Stopping backtest because tickstream has ended");
            let _ = internal_handle_tx.send(BacktestCommand::Stop);
            Ok::<(), ()>(())
        }).forget();

        let uuid = Uuid::new_v4();
        let handle = BacktestHandle {
            symbol: definition.symbol,
            backtest_type: definition.backtest_type,
            data_source: definition.data_source,
            endpoint: definition.data_dest,
            uuid: uuid.clone(),
            handle: external_handle_tx
        };

        // register the backtest's existence
        let mut backtest_list = self.running_backtests.lock().unwrap();
        backtest_list.push(handle);

        Ok(uuid)
    }

    /// Sends a command to a managed backtest
    pub fn send_backtest_cmd(&mut self, uuid: Uuid, cmd: BacktestCommand) -> Result<(), ()> {
        let mut backtest_list = self.running_backtests.lock().unwrap();
        for handle in backtest_list.iter_mut() {
            if handle.uuid == uuid {
                let ref sender = handle.handle;
                let _ = handle.handle.send(cmd);
                return Ok(())
            }
        }

        Err(())
    }
}

/// Creates a TickGenerator from a DataSource and symbol String
pub fn resolve_data_source(data_source: &DataSource, symbol: String) -> Box<TickGenerator> {
    match data_source {
        &DataSource::Flatfile => {
            Box::new(FlatfileReader{symbol: symbol.clone()}) as Box<TickGenerator>
        },
        &DataSource::Redis{ref host, ref channel} => {
            Box::new(
                RedisReader::new(symbol.clone(), host.clone(), channel.clone())
            ) as Box<TickGenerator>
        },
        &DataSource::Random => {
            Box::new(RandomReader::new(symbol.clone())) as Box<TickGenerator>
        },
    }
}

/// Returns true if the backtest has met a stop condition.
fn check_early_exit (
    t: &Tick, def: &BacktestDefinition, i: usize
) -> bool {
    if def.max_tick_n.is_some() &&
       def.max_tick_n.unwrap() <= i {
        return true
    } else if def.max_timestamp.is_some() &&
              def.max_timestamp.unwrap() <= t.timestamp {
        return true
    }

    false
}

/// What kind of method used to time the output of data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BacktestType {
    Fast{delay_ms: usize},
    Live,
}

/// Where to get the data to drive the backtest
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DataSource {
    Flatfile,
    Redis{host: String, channel: String},
    Random
}

/// Where to send the backtest's generated data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DataDest {
    Redis{host: String, channel: String},
    Console,
    Null
}

#[test]
fn backtest_n_early_exit() {
    let rx = algobot_util::transport::redis::sub_channel(CONF.redis_url, "test1_ii");

    let mut bt = Backtester::new();
    let definition = BacktestDefinition {
        max_tick_n: Some(10),
        max_timestamp: None,
        symbol: "TEST".to_string(),
        backtest_type: BacktestType::Fast{delay_ms: 0},
        data_source: DataSource::Random,
        data_dest: DataDest::Redis{
            host: CONF.redis_url.to_string(),
            channel: "test1_ii".to_string()
        },
        broker_settings: SimBrokerSettings::default(),
    };

    let uuid = bt.start_backtest(definition).unwrap();
    // backtest starts paused so resume it
    let _ = bt.send_backtest_cmd(uuid, BacktestCommand::Resume);
    let res = rx.wait().take(10).collect::<Vec<_>>();
    assert_eq!(res.len(), 10);
}

#[test]
fn backtest_timestamp_early_exit() {
    let rx = algobot_util::transport::redis::sub_channel(CONF.redis_url, "test2_ii");

    let mut bt = Backtester::new();
    let definition = BacktestDefinition {
        max_tick_n: None,
        max_timestamp: Some(8),
        symbol: "TEST".to_string(),
        backtest_type: BacktestType::Fast{delay_ms: 0},
        data_source: DataSource::Random,
        data_dest: DataDest::Redis{
            host: CONF.redis_url.to_string(),
            channel: "test2_ii".to_string()
        },
        broker_settings: SimBrokerSettings::default(),
    };

    let uuid = bt.start_backtest(definition).unwrap();
    // backtest starts paused so resume it
    let _ = bt.send_backtest_cmd(uuid, BacktestCommand::Resume);
    let res = rx.wait().take(8).collect::<Vec<_>>();
    assert_eq!(res.len(), 8);
}
