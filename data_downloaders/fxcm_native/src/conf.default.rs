//! Configuration for the FXCM Broker

pub struct Conf {
    pub fxcm_username: &'static str,
    pub fxcm_url: &'static str,
    pub fxcm_password: &'static str,
    pub fxcm_pin: Option<&'static str>,
    // postgres settings
    pub postgres_url: &'static str,
    pub postgres_port: usize,
    pub postgres_user: &'static str,
    pub postgres_password: &'static str,
    pub postgres_db: &'static str,
    // redis settings
    pub redis_host: &'static str,
}

pub const CONF: Conf = Conf {
    fxcm_username: "D102698627001",
    fxcm_password: "1576",
    fxcm_url: "http://www.fxcorporate.com/Hosts.jsp",
    fxcm_pin: Some("1234"),
    // postgres settings
    postgres_url: "localhost",
    postgres_port: 5432,
    postgres_user: "trading_bot",
    postgres_password: "password",
    postgres_db: "trading_bot",
    // redis settings
    redis_host: "redis://localhost",
};
