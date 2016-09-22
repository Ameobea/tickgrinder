// Copy this file to conf.rs and set values as appropriate
// to run the tick processor

pub struct Conf {
    // Redis config
    pub redis_url: &'static str,
    pub redis_control_channel: &'static str,
    pub redis_responses_channel: &'static str,
}

pub const CONF: Conf = Conf {
    // Redis config
    redis_url: "redis://127.0.0.1/",
    redis_control_channel: "control",
    redis_responses_channel: "responses",
};
