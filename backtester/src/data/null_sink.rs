//! Pipes data into the abyss.

#[allow(unused_imports)]
use test;

use algobot_util::trading::tick::Tick;

use data::TickSink;

pub struct NullSink {}

impl TickSink for NullSink {
    fn tick(&mut self, t: Tick) {}
}

/// I'd like to imagine this is optimized out but you never know...
#[bench]
fn null_sink(b: &mut test::Bencher) {
    let mut ns = NullSink{};
    let t = Tick::null();
    b.iter(|| ns.tick(t))
}
