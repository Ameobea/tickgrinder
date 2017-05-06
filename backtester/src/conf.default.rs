pub struct Conf {
    // A relative path to the directory where flatfile tick data is stored
    pub tick_data_dir: &'static str,
    // Redis config
    pub redis_url: &'static str,
    pub redis_control_channel: &'static str,
    pub redis_responses_channel: &'static str,
    // postgres settings
    pub postgres_url: &'static str,
    pub postgres_port: usize,
    pub postgres_user: &'static str,
    pub postgres_password: &'static str,
    pub postgres_db: &'static str,
}

pub const CONF: Conf = Conf {
    tick_data_dir: "../tick_data",
    // Redis config
    redis_url: "redis://localhost/",
    redis_control_channel: "control",
    redis_responses_channel: "responses",
    // postgres settings
    postgres_url: "localhost",
    postgres_port: 5432,
    postgres_user: "trading_bot",
    postgres_password: "password",
    postgres_db: "trading_bot",
};
