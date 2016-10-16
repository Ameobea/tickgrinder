//! Backtester module
//!
//! Plays back market data and executes strategies, providing a simulated broker and
//! account as well as statistics and data about the results of the strategy.

extern crate algobot_util;
extern crate futures;
extern crate uuid;
extern crate redis;

mod data;
mod conf;
mod backtest;

use uuid::Uuid;
use futures::Future;
use futures::stream::Stream;

use algobot_util::transport::command_server::{CommandServer, CsSettings};
use algobot_util::transport::redis::{sub_multiple, get_client};
use algobot_util::transport::commands::*;
use conf::CONF;

/// Starts the backtester module, initializing its interface to the rest of the platform
fn main() {
    let mut backtester = Backtester::new();
    backtester.listen();
}

struct Backtester {
    pub uuid: Uuid,
    pub cs: CommandServer
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
            cs: CommandServer::new(settings)
        }
    }

    /// Starts listening for commands from the rest of the platform
    pub fn listen(&mut self) {
        // subscribe to the command channels
        let rx = sub_multiple(CONF.redis_url, &[CONF.redis_control_channel, self.uuid.hyphenated().to_string().as_str()]);
        let mut redis_client = get_client(CONF.redis_url);

        rx.for_each(move |(_, msg)| {
            let wr_cmd = match WrappedCommand::from_str(msg.as_str()) {
                Ok(wr) => wr,
                Err(e) => {
                    println!("Unable to parse WrappedCommand from String: {:?}", e);
                    return Ok(())
                }
            };
            let res = match wr_cmd.cmd {
                Command::Ping => Response::Pong{ args: std::env::args().collect() },
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
}
