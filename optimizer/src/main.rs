//! Algobot 4 Optimizer
//! Created by Casey Primozic 2016-2016

#![feature(custom_derive, plugin, proc_macro, conservative_impl_trait, custom_derive, plugin, test)]

extern crate test;
extern crate uuid;
extern crate postgres;
extern crate redis;
extern crate futures;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate fxcm;

extern crate algobot_util;
extern crate sample as strat;

mod conf;

use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use uuid::Uuid;
use futures::stream::Stream;

use algobot_util::transport::redis::*;
use algobot_util::transport::commands::*;
use algobot_util::transport::command_server::{CommandServer, CsSettings};
use algobot_util::transport::postgres::PostgresConf;
use algobot_util::transport::query_server::QueryServer;
use algobot_util::strategies::Strategy;
use fxcm::FXCMNative;
use conf::CONF;

const CS_SETTINGS: CsSettings = CsSettings {
    redis_host: CONF.redis_host,
    responses_channel: CONF.redis_response_channel,
    conn_count: CONF.conn_senders,
    timeout: CONF.cs_timeout,
    max_retries: CONF.cs_max_retries
};

const PG_CONF: PostgresConf = PostgresConf {
    postgres_user: CONF.postgres_user,
    postgres_password: CONF.postgres_password,
    postgres_url: CONF.postgres_url,
    postgres_port: CONF.postgres_port,
    postgres_db: CONF.postgres_db
};

struct Optimizer {
    cs: CommandServer,
    uuid: Uuid,
}

impl Optimizer {
    pub fn new() -> Optimizer {
        let cs = CommandServer::new(CS_SETTINGS);
        let uuid = Uuid::new_v4();

        Optimizer {
            cs: cs,
            uuid: uuid,
        }
    }

    pub fn init(mut self) {
        // initialize the strategy
        let query_server = QueryServer::new(CONF.conn_senders, PG_CONF);
        let mut broker = FXCMNative::new(HashMap::new());
        let cs = self.cs.clone();
        thread::spawn(move || {
            let mut strat = strat::new(cs, query_server, &mut broker);
            strat.init();
        });

        let rx = sub_multiple(CONF.redis_host, &[self.uuid.hyphenated().to_string().as_str(), CONF.redis_commands_channel]);
        let client = get_client(CONF.redis_host);

        // TODO: Switch to send_forget once implemented
        let _ = self.cs.execute(Command::Ready{
            instance_type: "Optimizer".to_string(),
            uuid: self.uuid,
        }, CONF.redis_commands_channel.to_string());

        for msg in rx.wait() {
            let msg_string = msg.unwrap().1;
            let wr_msg_res = serde_json::from_str::<WrappedCommand>(&msg_string);
            if wr_msg_res.is_err() {
                println!("Unable to parse WrappedCommand from String: {:?}", &msg_string);
                continue
            }

            let wr_cmd = wr_msg_res.unwrap();
            let res = self.get_response(&wr_cmd.cmd);
            let wr_res = res.wrap(wr_cmd.uuid);
            let _ = send_response(&wr_res, &client, CONF.redis_response_channel);
        }
    }

    fn get_response(&mut self, cmd: &Command) -> Response {
        match cmd {
            &Command::Ping => Response::Pong{ args: vec![self.uuid.hyphenated().to_string()] },
            &Command::Type => Response::Info{ info: "Optimizer".to_string() },
            &Command::Kill => {
                thread::spawn(|| {
                    thread::sleep(Duration::from_secs(3));
                    println!("Optimizer shutting down now.");
                });
                Response::Info{ info: "Optimizer ending life in 3 seconds...".to_string() }
            },
            _ => Response::Error{status: "Optimizer doesn't recognize that command.".to_string() }
        }
    }
}

fn main() {
    Optimizer::new().init()
}
