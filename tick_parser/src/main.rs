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

mod datafield;
mod calc;
mod transport;
mod conf;
mod processor;
mod tests;

use std::env;

use futures::stream::Stream;
use uuid::Uuid;

use processor::Processor;
use conf::CONF;
use algobot_util::trading::tick::SymbolTick;
use algobot_util::transport::postgres::{get_client, reset_db, PostgresConf};
use algobot_util::transport::redis::sub_multiple;
use algobot_util::transport::commands::{Command, send_command};

fn handle_messages(symbol: String, uuid: Uuid) {
    let ticks_channel = CONF.redis_ticks_channel;
    let control_channel = CONF.redis_control_channel;
    let uuid_string = uuid.hyphenated().to_string();

    let mut processor = Processor::new(symbol, uuid.clone());

    let rx = sub_multiple(
        CONF.redis_url, &[ticks_channel, control_channel, uuid_string.as_str()]
    );

    let _ = send_command(&Command::Ready{
        instance_type: "Tick Processor".to_string(),
        uuid: uuid,
    }.wrap(), &processor.redis_client, CONF.redis_control_channel);

    for res in rx.wait() {
        let (channel, message) = res.unwrap();
        if channel == ticks_channel {
            processor.process(SymbolTick::from_json_string(message))
        } else if channel == uuid_string.as_str()
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
        _ => panic!("Wrong number of arguments provided!")
    }

    if CONF.reset_db_on_load {
        let pg_conf = PostgresConf {
            postgres_user: CONF.postgres_user,
            postgres_password: CONF.postgres_password,
            postgres_url: CONF.postgres_url,
            postgres_port: CONF.postgres_port,
            postgres_db: CONF.postgres_db
        };
        reset_db(&get_client(pg_conf).expect("Unable to get postgres client"), CONF.postgres_user)
            .expect("Unable to reset database");
        println!("Database reset");
    }

    // Start the listeners for everything and blocks
    handle_messages(symbol, uuid);
}
