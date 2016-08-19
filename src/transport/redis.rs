// Responsible for receiving live ticks and other kinds of data and
// sending calculations and results back.

use redis::{Client, PubSub};

use conf::CONF;

pub struct Tickstream {
    ps: PubSub
}

fn get_pubsub() -> PubSub {
    let client = match Client::open(CONF.redis_url) {
        Ok(c) => c,
        Err(_) => panic!("Could not connect to redis!")
    };
    let mut pubsub = match client.get_pubsub() {
        Ok(p) => p,
        Err(_) => panic!("Could not create pubsub for redis client!")
    };
    pubsub.subscribe(CONF.redis_ticks_channel)
        .expect("Could not subscribe to the ticks channel");
    return pubsub;
}

impl Tickstream {
    pub fn new() -> Tickstream {
        return Tickstream {
            ps: get_pubsub()
        }
    }

    pub fn get_tick(&mut self) -> String {
        let msg = match self.ps.get_message() {
            Ok(m) => m,
            Err(_) => panic!("Could not get message from pubsub!")
        };
        let payload: String = match msg.get_payload() {
            Ok(s) => s,
            Err(_) => panic!("Could not convert redis message to string!")
        };
        return payload;
    }
}
