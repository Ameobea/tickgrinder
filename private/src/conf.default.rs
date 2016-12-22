//! Configuration for the FXCM Broker

pub struct Conf {
    // postgres settings
    pub postgres_url: &'static str,
    pub postgres_port: usize,
    pub postgres_user: &'static str,
    pub postgres_password: &'static str,
    pub postgres_db: &'static str,
    // redis settings
    pub redis_host: &'static str,
    // Command settings
    pub commands_channel: &'static str,
    pub responses_channel: &'static str,
}

pub const CONF: Conf = Conf {
    // postgres settings
    postgres_url: "localhost",
    postgres_port: 5432,
    postgres_user: "trading_bot",
    postgres_password: "password",
    postgres_db: "trading_bot",
    // redis settings
    redis_host: "redis://localhost",
    // Command settings
    commands_channel: "control",
    responses_channel: "responses",
};
