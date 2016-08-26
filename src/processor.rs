// Tick processor
// Takes incoming ticks from Redis and performs various operations on them to help
// deterine a trading signal.  The main goal is to produce a result as quickly as
// possible, so non-essential operations should be deferred asynchronously.

use serde_json;

use tick::Tick;
use datafield::DataField;
use calc::sma::SMAList;
use transport::postgres::{get_client, init_tick_table};
use transport::query_server::QueryServer;
use conf::CONF;

pub struct Processor {
    pub ticks: DataField<Tick>,
    pub smas: SMAList,
    qs: QueryServer
}

#[derive(Serialize, Deserialize)]
enum Command {
    Restart,
    Shutdown,
    AddSMA{period: f64},
    RemoveSMA{period: f64},
}

fn parse_command(cmd: String) -> Result<Command, serde_json::Error> {
    serde_json::from_str::<Command>(cmd.as_str())
}

impl Processor {
    pub fn new(symbol: &str) -> Processor {
        // Create database connection and initialize some tables
        let client = get_client().expect("Could not connect to Postgres");

        println!("Successfully connected to Postgres");
        init_tick_table(symbol, &client);

        Processor {
            ticks: DataField::new(),
            smas: SMAList::new(),
            qs: QueryServer::new(CONF.database_conns)
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
        let cmd: Command = parse_command(raw_cmd)
            .expect("Unable to parse command");
        match cmd {
            Command::Restart => unimplemented!(),
            Command::Shutdown => unimplemented!(),
            Command::AddSMA{period: pd} => self.smas.add(pd),
            Command::RemoveSMA{period: pd} => self.smas.remove(pd)
        }
    }
}
