//! Logger module for the platform.  Ingests, processes, and forwards logs from all
//! of the platform's modules.

#![feature(test, slice_patterns)]

extern crate test;
extern crate uuid;
extern crate futures;
extern crate algobot_util;

use std::env;
use std::thread;
use std::process;
use std::time::Duration;

use futures::Stream;
use uuid::Uuid;

use algobot_util::transport::redis::*;
use algobot_util::transport::commands::*;
use algobot_util::transport::command_server::*;
use algobot_util::conf::CONF;

pub struct Logger {
    cs: CommandServer
}

impl Logger {
    pub fn new(uuid: Uuid) -> Logger {
        Logger {
            cs: CommandServer::new(uuid, "Logger"),
        }
    }

    /// Start listening for new log messages and other Commands from the platform
    pub fn listen(mut self, uuid: Uuid) {
        let rx = sub_multiple(
            CONF.redis_host,
            &[&uuid.hyphenated().to_string(), CONF.redis_control_channel, CONF.redis_log_channel]
        );

        let client = get_client(CONF.redis_host);

        // start loop of waiting for messages to process
        for msg_res in rx.wait() {
            let (_, wr_cmd_string) = msg_res.expect("Got error in listen loop");
            let wr_cmd_res = WrappedCommand::from_str(&wr_cmd_string);
            if wr_cmd_res.is_err() {
                self.cs.error(
                    Some("CommandDeserialization"),
                    &format!("Unable to convert str into WrappedCommand: {}", wr_cmd_string)
                );
            }
            let wr_cmd = wr_cmd_res.unwrap();

            let res_opt = match wr_cmd.cmd {
                Command::Log{msg} => {
                    self.store_log_msg(msg);
                    None
                },
                Command::Type => Some(Response::Info{info: String::from("Logger")}),
                Command::Ping => Some(Response::Pong{args: vec![uuid.hyphenated().to_string()]}),
                Command::Kill => {
                    thread::spawn(|| {
                        thread::sleep(Duration::from_secs(3));
                        process::exit(0);
                    });
                    Some(Response::Info{info: String::from("Logger shutting down in 3 seconds...")})
                },
                _ => None,
            };

            // send the response if there is a response to send
            if res_opt.is_some() {
                let _ = send_response(&res_opt.unwrap().wrap(wr_cmd.uuid), &client, CONF.redis_responses_channel);
            }
        }
    }

    /// Record received log message somewhere permanent
    pub fn store_log_msg(&mut self, msg: LogMessage) {
        unimplemented!();
    }
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let uuid: Uuid;

    match args.as_slice() {
        &[_, ref uuid_str] => {
            uuid = Uuid::parse_str(uuid_str.as_str())
                .expect("Unable to parse Uuid from supplied argument");
        },
        _ => panic!("Wrong number of arguments provided!  Usage: ./logger [uuid]"),
    }

    let logger = Logger::new(uuid);
    logger.listen(uuid);
}
