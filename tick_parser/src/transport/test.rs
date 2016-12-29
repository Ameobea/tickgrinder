use futures::Future;
use futures::stream::Stream;
use redis;
use uuid::Uuid;

use std::thread;
use std::time::Duration;

use algobot_util::transport;
use algobot_util::transport::redis::*;
use algobot_util::transport::commands::*;
use algobot_util::transport::postgres;
use algobot_util::transport::query_server::QueryServer;
use algobot_util::transport::command_server::*;
use algobot_util::trading::tick::{Tick, SymbolTick};
use algobot_util::conf::CONF;
use processor::Processor;

#[test]
fn postgres_tick_insertion() {
    let pg_conf = PostgresConf {
        postgres_user: CONF.postgres_user,
        postgres_password: CONF.postgres_password,
        postgres_url: CONF.postgres_host,
        postgres_port: CONF.postgres_port,
        postgres_db: CONF.postgres_db
    };
    let mut qs = QueryServer::new(5, pg_conf);
    for i in 0..10 {
        let t = Tick {timestamp: i, bid: 1, ask: 1};
        t.store("test0", &mut qs);
    }
    // todo 🔜: make sure they were actually inserted
    //      ^^ 3 months later
}

#[test]
fn postgres_db_reset() {
    let pg_conf = PostgresConf {
        postgres_user: CONF.postgres_user,
        postgres_password: CONF.postgres_password,
        postgres_url: CONF.postgres_host,
        postgres_port: CONF.postgres_port,
        postgres_db: CONF.postgres_db
    };
    let client = postgres::get_client(pg_conf).expect("5");
    postgres::reset_db(&client, CONF.postgres_user).expect("6");
}

/// Subscribe to Redis PubSub channel, then send some ticks
/// through and make sure they're stored and processed.
#[test]
fn tick_ingestion() {
    let mut processor = Processor::new("test8".to_string(), &Uuid::new_v4());
    let rx = sub_channel(CONF.redis_host, "TEST_ticks_ii");
    let mut client = get_client(CONF.redis_host);

    // send 5 ticks to through the redis channel
    for timestamp in 1..6 {
        let client = &mut client;
        let tick_string = format!("{{\"bid\": 1, \"ask\": 1, \"timestamp\": {}}}", timestamp);
        println!("{}", tick_string);
        redis::cmd("PUBLISH")
            .arg("TEST_ticks_ii")
            .arg(tick_string)
            .execute(client);
    }

    // process the 5 ticks
    for json_tick in rx.wait().take(5) {
        processor.process(Tick::from_json_string(json_tick.expect("unable to unwrap json_tick")));
    }
    // assert_eq!(processor.ticks.len(), 5);
    // TODO: Update to modern tick processing stuff
}

#[test]
fn command_server_broadcast() {
    let settings = CsSettings {
        redis_host: CONF.redis_host,
        responses_channel: "broadcast_test_res",
        conn_count: 3,
        timeout: 399,
        max_retries: 3
    };

    let mut cs = CommandServer::new(settings);
    let mut client = get_client(CONF.redis_host);
    let rx = sub_channel(CONF.redis_host, "broadcast_test_cmd");

    let cmd = Command::Ping;
    let responses_future = cs.broadcast(cmd, "broadcast_test_cmd".to_string());

    let recvd_cmd_str = rx.wait().next().unwrap().unwrap();
    let recvd_cmd = WrappedCommand::from_str(recvd_cmd_str.as_str()).unwrap();
    let res = Response::Pong{args: vec!("1".to_string(), "2".to_string())};
    for _ in 0..2 {
        redis::cmd("PUBLISH")
            .arg("broadcast_test_res")
            .arg(res.wrap(recvd_cmd.uuid).to_string().unwrap().as_str())
            .execute(&mut client);
    }

    let responses = responses_future.wait().unwrap();
    assert_eq!(responses.len(), 2);
    thread::sleep(Duration::new(3,0));
}
