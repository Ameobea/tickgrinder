use futures::Future;
use futures::stream::Stream;
use redis;

use transport;
use transport::postgres;
use transport::query_server::QueryServer;
use tick::Tick;
use conf::CONF;
use processor::Processor;

#[test]
fn postgres_tick_insertion() {
    let mut qs = QueryServer::new(5);
    for i in 0..10 {
        let t = Tick {timestamp: i, bid: 1f64, ask: 1f64};
        t.store("eurusd", &mut qs);
    }
    // todo 🔜: make sure they were actually inserted
}

#[test]
fn postgres_db_reset() {
    let client = postgres::get_client().unwrap();
    postgres::reset_db(&client).unwrap();
}

// Subscribe to Redis PubSub channel, then send some ticks
// through and make sure they're stored and processed.
#[test]
fn tick_ingestion() {
    let mut processor = Processor::new("temp1");
    let rx = transport::redis::sub_channel(CONF.redis_ticks_channel);
    let mut client = transport::redis::get_client();

    // send 5 ticks to through the redis channel
    for timestamp in 0..5 {
        let client = &mut client;
        let tick_string = format!("{{\"bid\": 1, \"ask\": 1, \"timestamp\": {}}}", timestamp);
        redis::cmd("PUBLISH")
            .arg(CONF.redis_ticks_channel)
            .arg(tick_string)
            .execute(client);
    }

    // process the 5 ticks
    for json_tick in rx.wait().take(5) {
        processor.process(Tick::from_json_string(json_tick.unwrap()));
    }
    assert_eq!(processor.ticks.len(), 5);
}

// Processor listens to commands and updates internals accordingly
// insert one SMA into the processor then remove it
#[test]
fn sma_commands() {
    let mut processor = Processor::new("temp2");
    let rx = transport::redis::sub_channel(CONF.redis_control_channel);
    let mut client = transport::redis::get_client();
    let command_string = "{\"AddSMA\": {\"period\": 32.34}}";
    assert_eq!(processor.smas.smas.len(), 0);

    redis::cmd("PUBLISH")
        .arg(CONF.redis_control_channel)
        .arg(command_string)
        .execute(&mut client);
    // block until the message is received and processed
    let msg = rx.wait().next();
    processor.execute_command(msg.unwrap().unwrap());
    assert_eq!(processor.smas.smas.len(), 1);

    let rx2 = transport::redis::sub_channel(CONF.redis_control_channel);
    let command_string = "{\"RemoveSMA\": {\"period\": 32.34}}";
    redis::cmd("PUBLISH")
        .arg(CONF.redis_control_channel)
        .arg(command_string)
        .execute(&mut client);
    let msg = rx2.wait().next();
    processor.execute_command(msg.unwrap().unwrap());
    assert_eq!(processor.smas.smas.len(), 0);
}
