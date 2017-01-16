//! Contains all the tests and benchmarks for the module.

#[allow(unused_imports)]
use super::*;

/// It should be an error to try to subscribe to a symbol that the SimBroker doesn't keep track of.
#[test]
fn sub_ticks_err() {
    let settings = SimBrokerSettings::default();

    let mut sim_b = SimBroker::new(settings, CommandServer::new(Uuid::new_v4(), "SimBroker Test"));
    let stream = sim_b.sub_ticks("TEST".to_string());
    assert!(stream.is_err());
}

/// How long it takes to unwrap the sender, send a message, and re-store the sender.
#[bench]
fn send_push_message(b: &mut test::Bencher) {
    let settings = SimBrokerSettings::default();
    let mut sim_b = SimBroker::new(settings, CommandServer::new(Uuid::new_v4(), "SimBroker Test"));
    let receiver = sim_b.get_stream().unwrap();
    thread::spawn(move ||{
        for _ in receiver.wait() {

        }
    });

    b.iter(|| {
        sim_b.push_msg(Ok(BrokerMessage::Success))
    })
}

/// Ticks sent to the SimBroker should be re-broadcast to the client.
#[test]
fn tick_retransmission() {
    use std::sync::mpsc;

    use futures::{Future, oneshot};

    use data::random_reader::RandomReader;
    use data::TickGenerator;
    use backtest::{NullMap, BacktestCommand};

    // oneshot with which to receive the tick sub channel
    let (channel_complete, channel_oneshot) = oneshot();

    thread::spawn(move || {
        // create the SimBroker
        let symbol = "TEST".to_string();
        let settings = SimBrokerSettings::default();
        let mut sim_b = SimBroker::new(settings, CommandServer::new(Uuid::new_v4(), "SimBroker Test"));
        let msg_stream = sim_b.get_stream();

        // create a random tickstream and register it to the SimBroker
        let mut gen = RandomReader::new(symbol.clone());
        let map = Box::new(NullMap {});
        let (tx, rx) = mpsc::sync_channel(5);
        let tick_stream = gen.get(map, rx);
        // start the random tick generator
        let _ = tx.send(BacktestCommand::Resume);

        // register the tickstream with the simbroker
        let res = sim_b.register_tickstream(symbol.clone(), tick_stream.unwrap(), false, 0);
        assert!(res.is_ok());

        // subscribe to ticks from the SimBroker for the test pair
        let subbed_ticks = sim_b.sub_ticks(symbol).unwrap();
        channel_complete.complete(subbed_ticks);

        // block this thread on the simbroker's simulation loop
        sim_b.init_sim_loop();
    });

    let subbed_ticks = channel_oneshot.wait().unwrap();
    let (c, o) = oneshot::<Vec<Tick>>();
    thread::spawn(move || {
        let res: Vec<Result<Tick, ()>> = subbed_ticks
            .wait().collect();
            // .take(10)
            // .map(|t| {
            //     println!("Received tick: {:?}", t);
            //     t.unwrap()
            // })
            // .collect();
        // signal once we've received all the ticks
        // c.complete(res);
    });

    // block until we've received all awaited ticks
    let res = o.wait().unwrap();
    assert_eq!(res.len(), 10);
}

#[test]
fn position_opening_closing_modification() {
    use futures::Future;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut sim = SimBroker::init(HashMap::new()).wait().unwrap().unwrap();

    let price = (0999, 1001);
    sim.oneshot_price_set(String::from("TEST"), price, false, 4);
    // TODO
}

#[test]
fn dynamic_base_rate_conversion() {
    use std::default::Default;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut settings = SimBrokerSettings::default();
    settings.fx_accurate_pricing = true;
    let mut sim = SimBroker::new(settings, cs);

    // wire tickstreams into the broker
    let (base_tx, base_rx) = unbounded::<Tick>();
    let base_pair    = String::from("EURUSD");
    let (foreign_tx, foreign_rx) = unbounded::<Tick>();
    let foreign_pair = String::from("EURJPY");
    sim.register_tickstream(base_pair.clone(), base_rx, true, 4).unwrap();
    sim.register_tickstream(foreign_pair.clone(), foreign_rx, true, 4).unwrap();

    base_tx.send(Tick {
        timestamp: 1,
        bid: 106143,
        ask: 106147
    }).wait().unwrap();
    foreign_tx.send(Tick {
        timestamp: 2,
        bid: 1219879,
        ask: 1219891,
    }).wait().unwrap();
    assert_eq!((106141, 106147), sim.get_price(&base_pair).unwrap());
    assert_eq!((1219879, 1219891), sim.get_price(&foreign_pair).unwrap());
    // TODO: Test reverses (EURUSD and USDEUR)
}

#[test]
fn oneshot_price_setting() {
    use futures::Future;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut sim = SimBroker::init(HashMap::new()).wait().unwrap().unwrap();

    let price = (0999, 1001);
    let sym = String::from("TEST");
    sim.oneshot_price_set(sym.clone(), price, false, 4);
    assert_eq!(price, sim.get_price(&sym).unwrap());
}

#[test]
fn oneshot_base_rate_conversion() {
    use futures::Future;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut sim = SimBroker::init(HashMap::new()).wait().unwrap().unwrap();

    sim.oneshot_price_set(String::from("EURUSD"), (106143, 106147), true, 5);
}

#[bench]
fn mutex_lock_unlock(b: &mut test::Bencher) {
    let amtx = Arc::new(Mutex::new(0));
    b.iter(move || {
        let _ = amtx.lock();
    })
}

#[bench]
fn small_string_hashmap_lookup(b: &mut test::Bencher) {
    let mut hm = HashMap::new();
    hm.insert(String::from("key1"), String::from("val1"));
    hm.insert(String::from("key2"), String::from("val2"));
    hm.insert(String::from("key3"), String::from("val3"));
    hm.insert(String::from("key4"), String::from("val4"));
    hm.insert(String::from("key5"), String::from("val5"));

    let lookup_key = String::from("key4");
    b.iter(|| hm.get(&lookup_key))
}
