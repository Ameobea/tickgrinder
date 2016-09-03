// Tick processor
// Takes incoming ticks from Redis and performs various operations on them to help
// deterine a trading signal.  The main goal is to produce a result as quickly as
// possible, so non-essential operations should be deferred asynchronously.

use serde_json;
use redis;
use algobot_util::transport::commands::*;

use tick::Tick;
use datafield::DataField;
use calc::sma::SMAList;
use transport::postgres::{get_client, init_tick_table};
use transport::query_server::QueryServer;
use algobot_util::transport::redis::get_client as get_redis_client;
use conf::CONF;

pub struct Processor {
    pub ticks: DataField<Tick>,
    pub smas: SMAList,
    qs: QueryServer,
    redis_client: redis::Client
}

pub fn send_response(res: &WrappedResponse, client: &redis::Client) {
    let ser = serde_json::to_string(res).expect("Couldn't serialize WrappedResponse into String");
    let res_str = ser.as_str();
    let _ = redis::cmd("PUBLISH")
        .arg(CONF.redis_responses_channel)
        .arg(res_str)
        .execute(client);
}

impl Processor {
    pub fn new(symbol: &str) -> Processor {
        // Create database connection and initialize some tables
        let pg_client = get_client().expect("Could not connect to Postgres");

        println!("Successfully connected to Postgres");
        init_tick_table(symbol, &pg_client);

        Processor {
            ticks: DataField::new(),
            smas: SMAList::new(),
            qs: QueryServer::new(CONF.database_conns),
            redis_client: get_redis_client(CONF.redis_url)
        }
    }

    // Called for each new tick received by the tick processor
    pub fn process(&mut self, t: Tick) {
        // Add to internal tick data field
        self.ticks.push(t);
        // Calculate smas
        self.smas.push_all(*self.ticks.last().unwrap());
        // Initialize async database store
        t.store(CONF.symbol, &mut self.qs);
    }

    pub fn execute_command(&mut self, raw_cmd: String) {
        let wrapped_cmd: WrappedCommand = parse_wrapped_command(raw_cmd);
        match wrapped_cmd.cmd {
            Command::Restart => unimplemented!(),
            Command::Shutdown => unimplemented!(),
            Command::AddSMA{period: pd} => {
                self.smas.add(pd);
                let wr = WrappedResponse{uuid: wrapped_cmd.uuid, res: Response::Ok};
                send_response(&wr, &self.redis_client);
            },
            Command::RemoveSMA{period: pd} => self.smas.remove(pd),
            Command::Ping => {
                let wr = WrappedResponse{uuid: wrapped_cmd.uuid, res: Response::Pong};
                send_response(&wr, &self.redis_client);
            }
        }
    }
}
