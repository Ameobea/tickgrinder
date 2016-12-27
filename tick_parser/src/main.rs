// Algobot 4 Tick Processor
// Casey Primozic, 2016-2016

#![feature(custom_derive, plugin, test, conservative_impl_trait, slice_patterns)]

extern crate redis;
extern crate futures;
extern crate serde_json;
extern crate postgres;
extern crate test;
extern crate uuid;
extern crate algobot_util;

mod transport;
mod processor;

use std::env;

use futures::stream::Stream;
use uuid::Uuid;

use processor::Processor;
use algobot_util::transport::postgres::{get_client, reset_db, PostgresConf};
use algobot_util::transport::redis::sub_multiple;
use algobot_util::transport::commands::{Command, send_command};
use algobot_util::conf::CONF;

const PG_CONF: PostgresConf = PostgresConf {
    postgres_user: CONF.postgres_user,
    postgres_password: CONF.postgres_password,
    postgres_url: CONF.postgres_host,
    postgres_port: CONF.postgres_port,
    postgres_db: CONF.postgres_db
};

struct TickProcessor {
    uuid: Uuid
}

impl TickProcessor {
    pub fn new(uuid: Uuid) -> TickProcessor {
        TickProcessor {
            uuid: uuid,
        }
    }

    /// Subscribes to Command channels
    pub fn listen(&self, symbol: String) {
        let control_channel = CONF.redis_control_channel;
        let uuid_string = self.uuid.hyphenated().to_string();

        let mut processor = Processor::new(symbol, &self.uuid);

        let rx = sub_multiple(
            CONF.redis_host, &[control_channel, uuid_string.as_str()]
        );

        let _ = send_command(&Command::Ready{
            instance_type: "Tick Processor".to_string(),
            uuid: self.uuid,
        }.wrap(), &processor.redis_client, CONF.redis_control_channel);

        for res in rx.wait() {
            let (channel, message) = res.unwrap();
            if channel == uuid_string.as_str()
                   || channel == control_channel {
                processor.execute_command(CONF.redis_responses_channel, message)
            } else {
                println!(
                    "Unexpected channel/message combination received: {},{}",
                    channel,
                    message
                );
            }
        }
    }
}

fn main() {
    // ./tick_processor uuid symbol
    let args = env::args().collect::<Vec<String>>();
    let uuid: Uuid;
    let symbol: String;

    match args.as_slice() {
        &[_, ref uuid_str, ref symbol_str] => {
            uuid = Uuid::parse_str(uuid_str.as_str())
                .expect("Unable to parse Uuid from supplied argument");
            symbol = symbol_str.to_string();
        }
        _ => panic!("Wrong number of arguments provided!  Usage: ./tick_processor [uuid] [symbol]")
    }

    if CONF.reset_db_on_load {
        reset_db(&get_client(PG_CONF).expect("Unable to get postgres client"), CONF.postgres_user)
            .expect("Unable to reset database");
        println!("Database reset");
    }

    let tp = TickProcessor::new(uuid);
    // Start the listeners for everything and blocks
    tp.listen(symbol);
    // the Tick Processor will now block until it receives messages from the platform that inform
    // it to subscribe to a broker's tick stream and start processing ticks.
}
