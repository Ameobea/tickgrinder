// COPY THIS TO conf.rs BEFORE RUNNING

//! Configuration for the optimizer to be set manually before startup

pub struct Conf {
    pub redis_commands_channel: &'static str,
    pub redis_response_channel: &'static str,
    pub redis_host: &'static str,
    pub command_timeout_ms: u64,
    pub max_command_retry_attempts: usize,
    pub conn_senders: usize
}

pub const CONF: Conf = Conf {
    redis_commands_channel: "commands",
    redis_response_channel: "responses",
    redis_host: "redis://localhost",
    command_timeout_ms: 3999,
    max_command_retry_attempts: 3,
    conn_senders: 5
};
