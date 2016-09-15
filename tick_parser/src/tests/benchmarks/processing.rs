use test::Bencher;

use datafield::DataField;
use algobot_util::tick::Tick;
use calc::sma::SimpleMovingAverage;

// insert a tick into a DataField
#[bench]
fn tick_insertion(b: &mut Bencher) {
    let t = Tick {bid: 1.123128412, ask: 1.123128402, timestamp: 1471291001837};
    let mut df: DataField<Tick> = DataField::new();

    b.iter(|| {
        let mut df = &mut df;
        df.push(t);
    });
}

// parse a JSON String into a Tick
#[bench]
fn json_to_tick(b: &mut Bencher) {
    b.iter(|| {
        let s: String = String::from("{\"bid\": 1.123128412, \"ask\": 1.123128402, \"timestamp\": 1471291001837}");
        Tick::from_json_string(s);
    });
}

#[bench]
fn sma_calculation(b: &mut Bencher) {
    let mut sma = SimpleMovingAverage::new(15f64);
    let mut timestamp = 1i64;

    b.iter(|| {
        sma.push(Tick{bid: 1.239123, ask: 1.12312, timestamp: timestamp});
        timestamp += 1;
    });
}
