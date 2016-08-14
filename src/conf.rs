pub struct Conf {
    pub redis_url: &'static str,
    pub redis_ticks_channel: &'static str
}

pub const CONF: Conf = Conf {
    redis_url: "redis://127.0.0.1/",
    redis_ticks_channel: "ticks"
};
