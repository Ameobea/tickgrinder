// Tick processor
// Takes incoming ticks from Redis and performs various operations on them to help
// deterine a trading signal.  The main goal is to produce a result as quickly as
// possible, so non-essential operations should be deferred asynchronously.

use std::{thread, process};
use std::time::Duration;

use redis;
use uuid::Uuid;
use algobot_util::transport::commands::*;

use datafield::DataField;
use calc::sma::SMAList;
use algobot_util::tick::{Tick, SymbolTick};
use algobot_util::transport::postgres::{get_client, init_tick_table, PostgresConf};
use algobot_util::transport::query_server::QueryServer;
use algobot_util::transport::redis::get_client as get_redis_client;
use conf::CONF;

pub struct Processor {
    pub uuid: Uuid,
    pub symbol: String,
    pub ticks: DataField<Tick>,
    pub smas: SMAList,
    qs: QueryServer,
    redis_client: redis::Client
}

impl Processor {
    pub fn new(symbol: String, uuid: Uuid) -> Processor {
        let pg_conf = PostgresConf {
            postgres_user: CONF.postgres_user,
            postgres_password: CONF.postgres_password,
            postgres_url: CONF.postgres_url,
            postgres_port: CONF.postgres_port,
            postgres_db: CONF.postgres_db
        };
        // Create database connection and initialize some tables
        let pg_client = get_client(pg_conf.clone()).expect("Could not connect to Postgres");

        println!("Successfully connected to Postgres");
        init_tick_table(symbol.as_str(), &pg_client, CONF.postgres_user);

        Processor {
            uuid: uuid,
            symbol: symbol,
            ticks: DataField::new(),
            smas: SMAList::new(),
            qs: QueryServer::new(CONF.qs_connections, pg_conf),
            redis_client: get_redis_client(CONF.redis_url)
        }
    }

    // Called for each new tick received by the tick processor
    pub fn process(&mut self, st: SymbolTick) {
        if st.symbol != self.symbol {
            return
        }

        let t = Tick::from_symboltick(st);
        // Add to internal tick data field
        self.ticks.push(t);
        // Calculate smas
        self.smas.push_all(*self.ticks.last().expect("Unable to pop tick from processor"));
        // Initialize async database store
        t.store(self.symbol.as_str(), &mut self.qs);
    }

    /// Handle an incoming Command, take action, and return a Response
    pub fn execute_command(&mut self, res_channel: &str, raw_cmd: String) {
        let wrapped_cmd: WrappedCommand = parse_wrapped_command(raw_cmd);
        let res = match wrapped_cmd.cmd {
            Command::Shutdown => unimplemented!(),
            Command::Kill => {
                // initiate suicide from another thread after a 3-second timeout
                thread::spawn(|| {
                    thread::sleep(Duration::from_secs(3));
                    println!("I can see the light...");
                    process::exit(0);
                });
                Response::Info{info: "Shutting down in 3 seconds...".to_string()}
            }
            Command::AddSMA{period: pd} => {
                self.smas.add(pd);
                Response::Ok
            },
            Command::RemoveSMA{period: pd} => {
                self.smas.remove(pd);
                Response::Ok
            },
            Command::Ping => {
                Response::Pong{uuid: self.uuid}
            },
            _ => {
                Response::Error{status: "Command not recognized".to_string()}
            }
        };

        let wr = res.wrap(wrapped_cmd.uuid);
        send_response(&wr, &self.redis_client, res_channel);
    }
}
