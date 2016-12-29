//! Logger module for the platform.  Ingests, processes, and forwards logs from all
//! of the platform's modules.

#![feature(test, slice_patterns)]

extern crate test;
extern crate uuid;
extern crate futures;
extern crate postgres;
extern crate algobot_util;
extern crate time;
extern crate serde_json;

use std::env;
use std::thread;
use std::process;
use std::time::Duration;

use futures::Stream;
use uuid::Uuid;

use algobot_util::transport::redis::{get_client as get_redis_client, sub_multiple};
use algobot_util::transport::commands::*;
use algobot_util::transport::command_server::*;
use algobot_util::transport::query_server::*;
use algobot_util::conf::CONF;

pub struct Logger {
    cs: CommandServer,
    qs: QueryServer,
}

impl Logger {
    pub fn new(uuid: Uuid) -> Logger {
        Logger {
            cs: CommandServer::new(uuid, "Logger"),
            qs: QueryServer::new(CONF.conn_senders),
        }
    }

    /// Start listening for new log messages and other Commands from the platform
    pub fn listen(mut self, uuid: Uuid) {
        let rx = sub_multiple(
            CONF.redis_host,
            &[&uuid.hyphenated().to_string(), CONF.redis_control_channel, CONF.redis_log_channel]
        );

        let client = get_redis_client(CONF.redis_host);

        let cs_clone = self.cs.clone();
        thread::spawn(move || {
            // give spawner a chance to ... spawn before sending Ready message
            thread::sleep(Duration::from_secs(1));

            cs_clone.send_forget(
                &Command::Ready{uuid: uuid, instance_type: String::from("Logger")},
                CONF.redis_control_channel
            );
        });

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
                    self.store_log_msg(&msg);
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
    pub fn store_log_msg(&mut self, msg: &LogMessage) {
        let query = gen_log_query(msg);

        self.qs.execute(query);
    }

    /// Set up the schemas of the database tables that will store log entries.
    pub fn init_persistance(&mut self) {
        let query = format!(
            "CREATE TABLE IF NOT EXISTS {}
            (
              ID SERIAL PRIMARY KEY,
              sender_instance text,
              message_type text,
              message text,
              level smallint,
              log_time double precision NOT NULL
            )
            WITH (
              OIDS=FALSE
            );",
            CONF.logger_persistance_table
        );

        self.qs.execute(query);
    }
}

fn gen_log_query(msg: &LogMessage) -> String {
    let t = time::get_time();
    let ts: f64 = t.sec as f64 + (t.nsec as f64 / 1000000000f64);
    format!(
        "INSERT INTO {}
        (sender_instance, message_type, message, level, log_time)
        VALUES('{}', '{}', '{}', {}, {});",
        CONF.logger_persistance_table,
        escape(&serde_json::to_string(&msg.sender).expect("Unable to serialize Instance")),
        escape(&msg.message_type),
        escape(&msg.message),
        level_to_int(msg.level.clone()),
        ts
    )
}

fn level_to_int(level: LogLevel) -> u8 {
    match level {
        LogLevel::Notice => 0,
        LogLevel::Debug => 1,
        LogLevel::Warning => 2,
        LogLevel::Error => 3,
        LogLevel::Critical => 4,
    }
}

/// Escapes a string so it can be made suitable for insertion into a query
fn escape(unescaped: &String) -> String {
    unescaped.replace("'", "''")
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

    let mut logger = Logger::new(uuid);
    logger.init_persistance();
    logger.listen(uuid);
}

#[bench]
fn logmessage_to_query(b: &mut test::Bencher) {
    let msg = LogMessage {
        message_type: String::from("General"),
        level: LogLevel::Notice,
        sender: Instance {uuid: Uuid::new_v4(), instance_type: String::from("Example Instance") },
        message: String::from("This is a test message that could be logged with the logger."),
    };

    b.iter(|| gen_log_query(&msg))
}

#[test]
fn test_logging() {
    let mut cs = CommandServer::new(Uuid::new_v4(), "Imaginary Test Instance");
    cs.notice(None, "Test log message!");
}
