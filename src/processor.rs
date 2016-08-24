// Tick processor
// Takes incoming ticks from Redis and performs various operations on them to help
// deterine a trading signal.  The main goal is to produce a result as quickly as
// possible, so non-essential operations should be deferred asynchronously.

use tick::Tick;
use datafield::DataField;
use calc::sma::SimpleMovingAverage;
use transport::postgres::{get_client, init_tick_table};
use transport::query_server::QueryServer;
use conf::CONF;

pub struct Processor {
    ticks: DataField<Tick>,
    sma: SimpleMovingAverage,
    qs: QueryServer
}

impl Processor {
    pub fn new() -> Processor {
        // Create database connection and initialize some tables
        let client = match get_client() {
            Ok(c) => c,
            Err(e) => panic!("Could not connect to Postgres: {:?}", e)
        };
        init_tick_table(CONF.symbol, &client);

        Processor {
            ticks: DataField::new(),
            sma: SimpleMovingAverage::new(15f64),
            qs: QueryServer::new(CONF.database_conns)
        }
    }

    // Called for each new tick received by the tick processor
    pub fn process(&mut self, t: Tick) {
        // Add to internal tick data field
        self.ticks.push(t);

        // Calculate sma
        self.sma.push(*self.ticks.last().unwrap());

        // Initialize async database store
        t.store(CONF.symbol, &mut self.qs);
    }
}
