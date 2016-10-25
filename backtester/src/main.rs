//! Backtester module
//!
//! Plays back market data and executes strategies, providing a simulated broker and
//! account as well as statistics and data about the results of the strategy.

#![feature(conservative_impl_trait, associated_consts, custom_derive, proc_macro)]
#![allow(unused_variables, dead_code)]

extern crate algobot_util;
extern crate futures;
extern crate uuid;
extern crate redis;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod data;
mod conf;
mod backtest;
mod sim_broker;

use std::sync::{Arc, Mutex};

use uuid::Uuid;
use futures::Future;
use futures::stream::{channel, Stream, Receiver};

use algobot_util::transport::command_server::{CommandServer, CsSettings};
use algobot_util::transport::redis::{sub_multiple, get_client};
use algobot_util::transport::commands::*;
use algobot_util::trading::tick::Tick;
use conf::CONF;
use backtest::*;
use data::*;

/// Starts the backtester module, initializing its interface to the rest of the platform
fn main() {
    let mut backtester = Backtester::new();
    backtester.listen();
}

#[derive(Clone)]
struct Backtester {
    pub uuid: Uuid,
    pub cs: CommandServer,
    pub running_backtests: Arc<Mutex<Vec<BacktestHandle>>>
}

impl Backtester {
    pub fn new() -> Backtester {
        let settings = CsSettings {
            conn_count: 2,
            redis_host: CONF.redis_url,
            responses_channel: CONF.redis_responses_channel,
            timeout: 2020,
            max_retries: 3
        };

        let uuid = Uuid::new_v4();

        Backtester {
            uuid: uuid,
            cs: CommandServer::new(settings),
            running_backtests: Arc::new(Mutex::new(Vec::new()))
        }
    }

    /// Starts listening for commands from the rest of the platform
    pub fn listen(&mut self) {
        // subscribe to the command channels
        let rx = sub_multiple(CONF.redis_url, &[CONF.redis_control_channel, self.uuid.hyphenated().to_string().as_str()]);
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
                        Response::Error{ status: format!("Can't parse backtest defition from String: {}", err_msg) }
                    } else {
                        let _def: BacktestDefinition = definition.unwrap();

                        // create a TickGenerator object
                        let mut temp;
                        let mut temp2;
                        let mut temp3;
                        let mut gen = match _def.data_source {
                            DataSource::Flatfile => {
                                temp = FlatfileReader{symbol: _def.symbol.clone()};
                                &mut temp as &mut TickGenerator
                            },
                            DataSource::Redis => {
                                temp2 = RedisReader{symbol: _def.symbol.clone()};
                                &mut temp2 as &mut TickGenerator
                            },
                            DataSource::Random => {
                                temp3 = RandomReader{symbol: _def.symbol.clone()};
                                &mut temp3 as &mut TickGenerator
                            },
                        };

                        // start the backtest and register a handle internally
                        let uuid = copy.start_backtest(
                            _def.symbol, _def.backtest_type, &mut *gen, _def.data_dest
                        );

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
        &mut self, symbol: String, backtest_type: BacktestType, src: &mut TickGenerator, endpoint: DataDest
    ) -> Result<Uuid, String> {
        // modify the source tickstream to add delay between the ticks or add some other kind of
        // advanced functionality to the way they're outputted
        let tickstream: Result<Receiver<Tick, ()>, String> = match backtest_type {
            BacktestType::Fast{delay_ms} => src.get(&FastMap{delay_ms: delay_ms}),
            BacktestType::Live => src.get(&LiveMap::new()),
        };

        // Create a TickSink that receives the output of the backtest
        let dst: &TickSink = match endpoint {
            DataDest::Redis => unimplemented!(),
            DataDest::Console => unimplemented!(),
        };

        let (handle_s, handle_r) = channel::<BacktestCommand, ()>();

        if tickstream.is_err() {
            return Err( format!("Error creating tickstream: {}", tickstream.err().unwrap()) )
        }

        let uuid = Uuid::new_v4();
        let handle = BacktestHandle {
            uuid: uuid.clone(),
            tickstream: tickstream.unwrap(),
            handle: handle_s
        };

        // register the backtest's existence
        let mut backtest_list = self.running_backtests.lock().unwrap();
        backtest_list.push(handle);

        Ok(uuid)
    }
}

/// What kind of method used to time the output of data
#[derive(Serialize, Deserialize, Debug)]
pub enum BacktestType {
    Fast{delay_ms: u64},
    Live,
}

/// Where to get the data to drive the backtest
#[derive(Serialize, Deserialize, Debug)]
pub enum DataSource {
    Flatfile,
    Redis,
    Random
}

/// Where to send the backtest's generated data
#[derive(Serialize, Deserialize, Debug)]
pub enum DataDest {
    Redis,
    Console,
}
