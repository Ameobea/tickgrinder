// Copy this file to conf.rs and set values as appropriate
// to run the tick processor

pub struct Conf {
    pub reset_db_on_load: bool, // wipe all stored data in postgres on app launch
    // General config
    pub database_conns: usize, // how many connections to open to the database
    // Redis config
    pub redis_url: &'static str,
    pub redis_control_channel: &'static str,
    pub redis_responses_channel: &'static str,
    // Postgres config
    pub postgres_url: &'static str,
    pub postgres_port: usize,
    pub postgres_user: &'static str,
    pub postgres_password: &'static str,
    pub postgres_db: &'static str,
    // Misc config
    pub qs_connections: usize
}

pub const CONF: Conf = Conf {
    reset_db_on_load: false,
    // General config
    database_conns: 10,
    // Redis config
    redis_url: "redis://127.0.0.1/",
    redis_control_channel: "control",
    redis_responses_channel: "responses",
    // Postgres config
    postgres_url: "localhost",
    postgres_port: 5432,
    postgres_user: "username",
    postgres_password: "password",
    postgres_db: "db_name",
    // Misc config
    qs_connections: 6
};
