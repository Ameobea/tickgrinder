// Tick processor
// Takes incoming ticks from Redis and performs various operations on them to help
// deterine a trading signal.  The main goal is to produce a result as quickly as
// possible, so non-essential operations should be deferred asynchronously.

use tick::Tick;
use datafield::DataField;

pub struct Processor {
    ticks: DataField<Tick>
}

impl Processor {
    pub fn new() -> Processor {
        Processor {
            ticks: DataField::new()
        }
    }

    // Add a new tick to be processed
    pub fn process(&mut self, t: Tick) {
        println!("Publishing tick: {:?}", t);
        self.ticks.push(t);
    }
}
