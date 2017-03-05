//! Pipes data into the abyss.

#[allow(unused_imports)]
use test;

use trading::tick::Tick;

use transport::tickstream::TickSink;

pub struct NullSink {}

impl TickSink for NullSink {
    #[allow(unused_variables)]
    fn tick(&mut self, t: Tick) {}
}

/// I'd like to imagine this is optimized out but you never know...
#[bench]
fn null_sink(b: &mut test::Bencher) {
    let mut ns = NullSink{};
    let t = Tick::null();
    b.iter(|| ns.tick(t))
}
