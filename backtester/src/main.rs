//! Backtester module
//!
//! Plays back market data and executes strategies, providing a simulated broker and
//! account as well as statistics and data about the results of the strategy.

#![feature(conservative_impl_trait)]
#![allow(unused_variables, dead_code)]

extern crate algobot_util;
extern crate futures;
extern crate uuid;
extern crate redis;

mod data;
mod conf;
mod backtest;

use std::sync::{Arc, Mutex};

use uuid::Uuid;
use futures::Future;
use futures::stream::Stream;

use algobot_util::transport::command_server::{CommandServer, CsSettings};
use algobot_util::transport::redis::{sub_multiple, get_client};
use algobot_util::transport::commands::*;
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
        let copy = self.clone();

        rx.for_each(move |(_, msg)| {
            let wr_cmd = match WrappedCommand::from_str(msg.as_str()) {
                Ok(wr) => wr,
                Err(e) => {
                    println!("Unable to parse WrappedCommand from String: {:?}", e);
                    return Ok(())
                }
            };

            let res = match wr_cmd.cmd {
                Command::Ping => Response::Pong{ args: vec![copy.uuid.hyphenated().to_string()] },
                Command::Type => Response::Info{ info: "Backtester".to_string() },
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
    fn start_backtest<G, S>(
        &mut self, symbol: String, backtest_type: BacktestType, data_source: G, endpoint: S
    )
            where G:TickGenerator, S:TickSink {
        let backtest = Backtest::new(symbol, backtest_type, data_source);
        let handle: BacktestHandle = backtest.init(endpoint);

        // register the backtest's existence
        let mut backtest_list = self.running_backtests.lock().unwrap();
        backtest_list.push(handle);
    }
}
