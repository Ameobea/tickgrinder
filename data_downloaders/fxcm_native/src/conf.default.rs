//! Configuration for the FXCM Broker

// Copy this to conf.rs once you've filled it with your desired settings

pub struct Conf {
    pub fxcm_username: &str,
    pub fxcm_password: &str,
    pub fxcm_url: &str,
    pub fxcm_pin: Option<&str>,
    // postgres settings
    pub postgres_host: &str,
    pub postgres_user: &str,
    pub postgres_password: &str,
    pub postgres_database: &str,
    // redis settings
    pub redis_host: &str,
}

pub const CONF: Conf = Conf {
    fxcm_username: "Your_username",
    fxcm_password: "Your_password",
    fxcm_url: "http://fxcorporate.com/Hosts.jsp",
    fxcm_pin: Some("1234"),
    // postgres settings
    postgres_host: "localhost",
    postgres_user: "username",
    postgres_password: "password",
    postgres_database: "database",
    // redis settings
    redis_host: "redis://localhost",
}
