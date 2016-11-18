//! Configuration for the FXCM Broker

// Copy this to conf.rs once you've filled it with your desired settings

pub struct Conf {
    pub Fxcm_username: &'static str,
    pub Fxcm_password: &'static str,
    pub Fxcm_pin: Option<&'static str>,
}

pub const CONF: Conf = Conf {
    Fxcm_username: "Your_username",
    Fxcm_password: "Your_password",
    Fxcm_pin: "1234",
}
