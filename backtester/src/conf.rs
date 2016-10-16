pub struct Conf {
    // A relative path to the directory where flatfile tick data is stored
    pub tick_data_dir: &'static str,
    // Redis config
    pub redis_url: &'static str,
    pub redis_control_channel: &'static str,
    pub redis_responses_channel: &'static str,
}

pub const CONF: Conf = Conf {
    tick_data_dir: "../tick_data",
    // Redis config
    redis_url: "redis://chitara.ameo.link/",
    redis_control_channel: "control",
    redis_responses_channel: "responses",
};
