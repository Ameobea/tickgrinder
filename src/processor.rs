// Tick processor
// Takes incoming ticks from Redis and performs various operations on them to help
// deterine a trading signal.  The main goal is to produce a result as quickly as
// possible, so non-essential operations should be deferred asynchronously.

use tick::Tick;
use datafield::DataField;
use calc::sma::SimpleMovingAverage;

pub struct Processor {
    ticks: DataField<Tick>,
    sma: SimpleMovingAverage
}

impl Processor {
    pub fn new() -> Processor {
        Processor {
            ticks: DataField::new(),
            sma: SimpleMovingAverage::new(15f64),
        }
    }

    // Add a new tick to be processed
    pub fn process(&mut self, t: Tick) {
        self.ticks.push(t);

        // sma
        let avg = self.sma.push(*self.ticks.last().unwrap());
        println!("15-second average: {:?}", avg);
    }
}
