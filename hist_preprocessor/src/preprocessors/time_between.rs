//! Calculates the average time between ticks on different time periods.

use postgres::Connection;

use preprocessor::Preprocessor;
use conf::CONF;

use algobot_util::trading::tick::*;
use algobot_util::transport::postgres::*;

pub struct TimeBetween {
    connection: Connection,
    last_tick: Tick,
}

impl Preprocessor for TimeBetween {
    fn process(&mut self, t: Tick) {
        if self.
    }
}

impl TimeBetween {
    pub fn new(window_size: usize) -> TimeBetween {
        let pgc = PostgresConf {
            postgres_db: CONF.postgres_db,
            postgres_password: CONF.postgres_password,
            postgres_port: CONF.postgres_port,
            postgres_user: CONF.postgres_user,
            postgres_url: CONF.postgres_url,
        };

        TimeBetween {
            connection: get_client(pgc).unwrap(),
            last_tick: Tick::null(),
        }
    }

    /// Stores the results of the transformation in the database
    pub fn store(&mut self) {

    }
}
