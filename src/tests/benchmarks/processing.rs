use test::Bencher;

use datafield::DataField;
use tick::Tick;

#[bench]
// insert a tick into a DataField
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
        Tick::from_string(s).unwrap();
    });
}
