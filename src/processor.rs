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
use transport::redis::get_client as get_redis_client;
use conf::CONF;

pub struct Processor {
    pub ticks: DataField<Tick>,
    pub smas: SMAList,
    qs: QueryServer,
    redis_client: redis::Client
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
            redis_client: get_redis_client()
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
            Command::AddSMA{period: pd} => self.smas.add(pd),
            Command::RemoveSMA{period: pd} => self.smas.remove(pd),
            Command::Ping => redis::cmd("PUBLISH")
                .arg("responses").arg(serde_json::to_string(&WrappedResponse{uuid: wrapped_cmd.uuid, res: Response::Pong}).unwrap().as_str()).execute(&self.redis_client)
        }
    }
}
