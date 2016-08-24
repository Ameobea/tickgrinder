// functions for communicating with the postgresql database

use postgres::{Connection, SslMode};
use postgres::error;

use conf::CONF;

pub fn get_client() -> Result<Connection, error::ConnectError> {
    let conn_string = format!("postgres://{}:{}@{}:{}/{}",
        CONF.postgres_user,
        CONF.postgres_password,
        CONF.postgres_url,
        CONF.postgres_port,
        CONF.postgres_db
    );

    Connection::connect(conn_string.as_str(), SslMode::None)
}

/***************************
   TICK-RELATED FUNCTIONS
***************************/

// Creates a new table for ticks with given symbol
pub fn init_tick_table(symbol: &str, client: &Connection) {
    let query1 = format!(
    "CREATE TABLE IF NOT EXISTS ticks_{}
    (
      tick_time bigint NOT NULL PRIMARY KEY UNIQUE,
      bid double precision NOT NULL,
      ask double precision NOT NULL
    )
    WITH (
      OIDS=FALSE
    );", symbol);
    let query2 = format!(
    "ALTER TABLE ticks_{}
      OWNER TO {};", symbol, CONF.postgres_user);
    client.execute(query1.as_str(), &[]).expect("Unable to query postgres to set up tick table");
    client.execute(query2.as_str(), &[]).expect("Unable to query postgres to set up tick table");
}

/***************************
  ADMINISTRATIVE FUNCTIONS
***************************/

// Drops all tables in the database, resetting it to defaults
pub fn reset_db(client: &Connection) -> Result<(), error::Error> {
    let query = format!("DROP SCHEMA public CASCADE;
        CREATE SCHEMA public AUTHORIZATION {};
        GRANT ALL ON SCHEMA public TO {};", CONF.postgres_user, CONF.postgres_user);
    client.batch_execute(query.as_str())
}
