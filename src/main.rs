// Algobot 3, Rust Version
// Casey Primozic, 2016-2016

mod datafield;
mod calc;
mod tick;

use std::slice::Iter;

use datafield::DataField;
use calc::sma::SimpleMovingAverage as SMA;
use tick::Tick;

fn main() {
    let mut tf = DataField::<Tick>::new();
    tf.data.push(Tick{price: 23f64, timestamp: 1470533189000i64});
    tf.data.push(Tick{price: 23f64, timestamp: 1470533191010i64});
    tf.data.push(Tick{price: 23.23894f64, timestamp: 1470533192410i64});

    for period in [3, 5].iter() {
        println!("Moving average with period {}", period);

        let mut sma = SMA::new(*period as i64);
        let mut test: Option<f64> = Some(0f64);
        for t in tf.data.iter() {
            test = sma.push(t);
        }
        println!("{:?}", test);
    }
}
