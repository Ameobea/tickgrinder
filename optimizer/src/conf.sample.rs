//! Configuration for the optimizer to be set manually before startup

// COPY THIS TO conf.rs AND FILL IN CORRECT VALUES BEFORE RUNNING

pub struct Conf {
    // Redis configuration
    pub redis_commands_channel: &'static str,
    pub redis_response_channel: &'static str,
    pub redis_host: &'static str,
    // CommandServer configuration
    pub cs_timeout: u64,
    pub cs_max_retries: usize,
    pub conn_senders: usize,
    // Postgres configuration
    pub postgres_url: &'static str,
    pub postgres_user: &'static str,
    pub postgres_password: &'static str,
    pub postgres_port: usize,
    pub postgres_db: &'static str
}

pub const CONF: Conf = Conf {
    // Redis configuration
    redis_commands_channel: "commands",
    redis_response_channel: "responses",
    redis_host: "redis://localhost",
    // CommandServer configuration
    cs_timeout: 3999,
    cs_max_retries: 3,
    conn_senders: 5,
    // Postgres configuration
    postgres_url: "localhost",
    postgres_user: "username",
    postgres_password: "password",
    postgres_port: 5432,
    postgres_db: "botdb"
};
