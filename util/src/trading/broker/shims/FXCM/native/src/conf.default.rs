//! Configuration for the FXCM Broker

// Copy this to conf.rs once you've filled it with your desired settings

pub struct Conf {
    pub fxcm_username: &'static str,
    pub fxcm_password: &'static str,
    pub fxcm_pin: Option<&'static str>,
}

pub const CONF: Conf = Conf {
    fxcm_username: "Your_username",
    fxcm_password: "Your_password",
    fxcm_pin: Some("1234"),
};
