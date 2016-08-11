// Responsible for receiving live ticks and other kinds of data and
// sending calculations and results back.

extern crate redis;
extern crate tokio;

use conf::conf;

pub fn get_tick() -> String {
    let client = match redis::Client::open(conf.redis_url) {
        Ok(c) => c,
        Err(e) => panic!("Could not connect to redis!")
    };
    let mut pubsub = match client.get_pubsub() {
        Ok(p) => p,
        Err(e) => panic!("Could not create pubsub for redis client!")
    };
    pubsub.subscribe(conf.redis_ticks_channel);

    loop {
        let msg = match pubsub.get_message() {
            Ok(m) => m,
            Err(e) => panic!("Could not get message from pubsub!")
        };
        let payload: String = match msg.get_payload() {
            Ok(s) => s,
            Err(e) => panic!("Could not convert redis message to string!")
        };
        println!("channel '{}': {}", msg.get_channel_name(), payload);
        return payload;
    }
}
