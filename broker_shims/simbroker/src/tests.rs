//! Contains all the tests and benchmarks for the module.

#[allow(unused_imports)]
use super::*;

/// It should be an error to try to subscribe to a symbol that the SimBroker doesn't keep track of.
#[test]
fn sub_ticks_err() {
    let mut sim_client = SimBrokerClient::init(HashMap::new()).wait().unwrap().unwrap();
    let stream = sim_client.sub_ticks("TEST".to_string());
    assert!(stream.is_err());
}

/// How long it takes to unwrap the sender, send a message, and re-store the sender.
#[bench]
fn send_push_message(b: &mut test::Bencher) {
    use futures::sync::mpsc::unbounded;

    let settings = SimBrokerSettings::default();
    let (_, dummy_rx) = unbounded();
    let mut sim_b = SimBroker::new(settings, CommandServer::new(Uuid::new_v4(), "SimBroker Test"), dummy_rx);
    let receiver = sim_b.push_stream_recv.take().unwrap().boxed();
    thread::spawn(move ||{
        for _ in receiver.wait() {

        }
    });

    b.iter(|| {
        sim_b.push_msg(Ok(BrokerMessage::Success))
    })
}

#[test]
fn position_opening_closing_modification() {
    use futures::Future;

    let mut sim = SimBrokerClient::init(HashMap::new()).wait().unwrap().unwrap();
    let price = (0999, 1001);
    sim.oneshot_price_set(String::from("TEST"), price, false, 4);
    // TODO
}

#[test]
fn dynamic_base_rate_conversion() {
    use std::default::Default;

    let mut hm = HashMap::new();
    hm.insert(String::from("fx_accurate_pricing"), String::from("true"));
    let mut sim_client = SimBrokerClient::init(hm).wait().unwrap().unwrap();

    // wire tickstreams into the broker
    let (mut base_tx, base_rx) = unbounded::<Tick>();
    let base_pair    = String::from("EURUSD");
    let (mut foreign_tx, foreign_rx) = unbounded::<Tick>();
    let foreign_pair = String::from("EURJPY");

    base_tx = base_tx.send(Tick {timestamp: 1, bid: 106143, ask: 106147}).wait().unwrap();
    base_tx = base_tx.send(Tick {timestamp: 3, bid: 106143, ask: 106147}).wait().unwrap();
    base_tx = base_tx.send(Tick {timestamp: 5, bid: 106143, ask: 106147}).wait().unwrap();
    foreign_tx = foreign_tx.send(Tick {timestamp: 2, bid: 1219879, ask: 1219891}).wait().unwrap();
    foreign_tx = foreign_tx.send(Tick {timestamp: 4, bid: 1219879, ask: 1219891}).wait().unwrap();
    foreign_tx = foreign_tx.send(Tick {timestamp: 6, bid: 1219879, ask: 1219891}).wait().unwrap();

    sim_client.register_tickstream(base_pair.clone(), base_rx, true, 4).unwrap();
    sim_client.register_tickstream(foreign_pair.clone(), foreign_rx, true, 4).unwrap();

    sim_client.init_sim_loop();

    // TODO: Sub prices and submit orders
    // TODO: Test reverses (EURUSD and USDEUR)
}

#[test]
fn oneshot_price_setting() {
    use futures::Future;

    let mut sim_client = SimBrokerClient::init(HashMap::new()).wait().unwrap().unwrap();

    let price = (0999, 1001);
    let sym = String::from("TEST");
    sim_client.oneshot_price_set(sym.clone(), price, false, 4);
    // TODO
}

#[test]
fn oneshot_base_rate_conversion() {
    use futures::Future;

    let cs = CommandServer::new(Uuid::new_v4(), "SimBroker Test");
    let mut sim_client = SimBrokerClient::init(HashMap::new()).wait().unwrap().unwrap();

    sim_client.oneshot_price_set(String::from("EURUSD"), (106143, 106147), true, 5);
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

#[test]
fn decimal_conversion() {
    assert_eq!(1000, convert_decimals(0100, 2, 3));
    assert_eq!(0999, convert_decimals(9991, 4, 3));
    assert_eq!(0010, convert_decimals(1000, 3, 1));
    assert_eq!(0000, convert_decimals(0000, 8, 2));
    assert_eq!(0001, convert_decimals(0001, 3, 3));
}

/// Make sure that the ordering of `QueueItem`s is reversed as it should be.
#[test]
fn reverse_event_ordering() {
    let item1 = QueueItem {
        timestamp: 5,
        unit: WorkUnit::NewTick(0, Tick::null()),
    };
    let item2 = QueueItem {
        timestamp: 6,
        unit: WorkUnit::NewTick(0, Tick::null()),
    };

    assert!(item2 < item1);
}

#[bench]
fn symbols_contains(b: &mut test::Bencher) {
    let mut symbols = Symbols::new(CommandServer::new(Uuid::new_v4(), "SimBroker Symbols Benchmark"));
    let name = String::from("TEST");
    let symbol = Symbol::new_oneshot((99, 103), true, 2, name.clone());
    let name_clone = name.clone();
    symbols.add(name, symbol);
    b.iter(|| symbols.contains(&name_clone))
}
