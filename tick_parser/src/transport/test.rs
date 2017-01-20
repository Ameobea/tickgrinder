use futures::Future;
use futures::stream::Stream;
use redis;
use uuid::Uuid;

use std::thread;
use std::time::Duration;

use tickgrinder_util::transport;
use tickgrinder_util::transport::redis::*;
use tickgrinder_util::transport::commands::*;
use tickgrinder_util::transport::postgres;
use tickgrinder_util::transport::query_server::QueryServer;
use tickgrinder_util::transport::command_server::*;
use tickgrinder_util::trading::tick::{Tick, SymbolTick};
use tickgrinder_util::conf::CONF;
use processor::Processor;

#[test]
fn postgres_tick_insertion() {
    let mut qs = QueryServer::new(5);
    for i in 0..10 {
        let t = Tick {timestamp: i, bid: 1, ask: 1};
        t.store("test0", &mut qs);
    }
    // todo 🔜: make sure they were actually inserted
    //      ^^ 3 months later
}

#[test]
fn postgres_db_reset() {
    let client = postgres::get_client().unwrap();
    postgres::reset_db(&client, CONF.postgres_user).unwrap();
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
    use std::str::FromStr;

    let cmds_channel_string = String::from("test_channel_998");
    let mut cs = CommandServer::new(Uuid::new_v4(), "Tick Processor Test");
    let mut client = get_client(CONF.redis_host);
    let rx = sub_channel(CONF.redis_host, &cmds_channel_string);

    let cmd = Command::Ping;
    let responses_future = cs.broadcast(cmd, cmds_channel_string);

    let recvd_cmd_str = rx.wait().next().unwrap().unwrap();
    let recvd_cmd = WrappedCommand::from_str(recvd_cmd_str.as_str()).unwrap();
    let res = Response::Pong{args: vec!("1".to_string(), "2".to_string())};
    for _ in 0..2 {
        redis::cmd("PUBLISH")
            .arg(CONF.redis_responses_channel)
            .arg(res.wrap(recvd_cmd.uuid).to_string().unwrap().as_str())
            .execute(&mut client);
    }

    let responses = responses_future.wait().unwrap();
    assert_eq!(responses.len(), 2);
    thread::sleep(Duration::new(3,0));
}
