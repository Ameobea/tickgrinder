// Copy this file to conf.rs and set values as appropriate
// to run the tick processor

pub struct Conf {
    pub node_binary_path: &'static str,
    // if false, takes control of straggler modules instead of killing them
    pub kill_stragglers: bool,
    // Redis config
    pub redis_url: &'static str,
    pub redis_control_channel: &'static str,
    pub redis_responses_channel: &'static str,
    // path to the dist directory created by running `make install` in the project root directory
    pub dist_path: &'static str,
}

pub const CONF: Conf = Conf {
    // Use version 4.2.0; using higher versions bind to ipv6 causing issues
    // TODO: Fix
    node_binary_path: "/home/casey/.nvm/versions/node/v4.2.0/bin/node",
    kill_stragglers: true,
    // Redis config
    redis_url: "redis://127.0.0.1/",
    redis_control_channel: "control",
    redis_responses_channel: "responses",
    dist_path: "/home/user/bot4/dist/", // needs trailing slash
};
