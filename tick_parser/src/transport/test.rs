use futures::Future;
use futures::stream::Stream;
use redis;

use algobot_util::transport;
use algobot_util::transport::postgres::{self, PostgresConf};
use algobot_util::transport::query_server::QueryServer;
use algobot_util::tick::{Tick, SymbolTick};
use conf::CONF;
use processor::Processor;
use algobot_util::transport::redis::*;

#[test]
fn postgres_tick_insertion() {
    let pg_conf = PostgresConf {
        postgres_user: CONF.postgres_user,
        postgres_password: CONF.postgres_password,
        postgres_url: CONF.postgres_url,
        postgres_port: CONF.postgres_port,
        postgres_db: CONF.postgres_db
    };
    let mut qs = QueryServer::new(5, pg_conf);
    for i in 0..10 {
        let t = Tick {timestamp: i, bid: 1f64, ask: 1f64};
        t.store("eurusd", &mut qs);
    }
    // todo 🔜: make sure they were actually inserted
}

#[test]
fn postgres_db_reset() {
    let pg_conf = PostgresConf {
        postgres_user: CONF.postgres_user,
        postgres_password: CONF.postgres_password,
        postgres_url: CONF.postgres_url,
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
    let mut processor = Processor::new("eurusd".to_string());
    let rx = sub_channel(CONF.redis_url, CONF.redis_ticks_channel);
    let mut client = get_client(CONF.redis_url);

    // send 5 ticks to through the redis channel
    for timestamp in 1..6 {
        let client = &mut client;
        let tick_string = format!("{{\"symbol\": \"eurusd\", \"bid\": 1, \"ask\": 1, \"timestamp\": {}}}", timestamp);
        redis::cmd("PUBLISH")
            .arg(CONF.redis_ticks_channel)
            .arg(tick_string)
            .execute(client);
    }

    // process the 5 ticks
    for json_tick in rx.wait().take(5) {
        processor.process(SymbolTick::from_json_string(json_tick.expect("unable to unwrap json_tick")));
    }
    assert_eq!(processor.ticks.len(), 5);
}

/// Processor listens to commands and updates internals accordingly
/// insert one SMA into the processor then remove it
#[test]
fn sma_commands() {
    let mut processor = Processor::new("temp2".to_string());
    let rx = sub_channel(CONF.redis_url, CONF.redis_control_channel);
    let mut client = get_client(CONF.redis_url);
    let command_string = "{\"uuid\":\"2f663301-5b73-4fa0-b231-09ab196ec5fd\",\
        \"cmd\":{\"AddSMA\":{\"period\":5.2342}}}";
    assert_eq!(processor.smas.smas.len(), 0);

    redis::cmd("PUBLISH")
        .arg(CONF.redis_control_channel)
        .arg(command_string)
        .execute(&mut client);
    // block until the message is received and processed
    let msg = rx.wait().next();
    processor.execute_command(msg.expect("1").expect("2"));
    assert_eq!(processor.smas.smas.len(), 1);

    let rx2 = sub_channel(CONF.redis_url, CONF.redis_control_channel);
    let command_string = "{\"uuid\":\"2f663301-5b73-4fa0-b201-09ab196ec5fd\",\
        \"cmd\":{\"RemoveSMA\":{\"period\":5.2342}}}";
    redis::cmd("PUBLISH")
        .arg(CONF.redis_control_channel)
        .arg(command_string)
        .execute(&mut client);
    let msg = rx2.wait().next();
    processor.execute_command(msg.expect("3").expect("4"));
    assert_eq!(processor.smas.smas.len(), 0);
}
