// COPY THIS TO conf.rs BEFORE RUNNING

//! Configuration for the optimizer to be set manually before startup

pub struct Conf {
    pub redis_commands_channel: &'static str,
    pub redis_response_channel: &'static str,
    pub redis_host: &'static str,
    // CommandServer Configuration
    pub cs_timeout: u64,
    pub cs_max_retries: usize,
    pub conn_senders: usize
}

pub const CONF: Conf = Conf {
    redis_commands_channel: "commands",
    redis_response_channel: "responses",
    redis_host: "redis://localhost",
    // CommandServer Configuration
    cs_timeout: 3999,
    cs_max_retries: 3,
    conn_senders: 5
};
