// Copy this file to conf.rs and set values as appropriate
// to run the tick processor

pub struct Conf {
    // General config
    pub symbol: &'static str, // symbol of ticks this processor will monitor
    pub database_conns: usize, // how many connections to open to the database
    // Redis config
    pub redis_url: &'static str,
    pub redis_ticks_channel: &'static str,
    // Postgres config
    pub postgres_url: &'static str,
    pub postgres_port: i32,
    pub postgres_user: &'static str,
    pub postgres_password: &'static str,
    pub postgres_db: &'static str
}

pub const CONF: Conf = Conf {
    // General config
    symbol: "EURUSD",
    database_conns: 10,
    // Redis config
    redis_url: "redis://127.0.0.1/",
    redis_ticks_channel: "ticks",
    // Postgres config
    postgres_url: "localhost",
    postgres_port: 5432,
    postgres_user: "username",
    postgres_password: "password",
    postgres_db: "db_name"
};
