//! Contains all private indicators you may devise for your system.

use algobot_util::transport::postgres::*;

use conf::CONF;

mod sma;

pub const PG_CONF: PostgresConf = PostgresConf {
    postgres_user: CONF.postgres_user,
    postgres_db: CONF.postgres_db,
    postgres_password: CONF.postgres_password,
    postgres_port: CONF.postgres_port,
    postgres_url: CONF.postgres_url,
};

pub use self::sma::Sma;
