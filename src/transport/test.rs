#[allow(unused_imports)]
use transport::query_server::QueryServer;
#[allow(unused_imports)]
use tick::Tick;

#[test]
fn postgres_tick_insertion() {
    let mut qs = QueryServer::new(5);
    for i in 0..10 {
        let t = Tick {timestamp: i, bid: 1f64, ask: 1f64};
        t.store("eurusd", &mut qs);
    }
    // todo 🔜: make sure they were actually inserted
}
