// Algobot 3, Rust Version
// Casey Primozic, 2016-2016

mod datafield;
mod calc;

use datafield::tickfield::TickField;
use datafield::tickfield::Tick;
use calc::sma::SimpleMovingAverage as SMA;

fn main() {
    let mut tf = TickField::new("USDCAD");
    tf.push(Tick{price: 23f64, timestamp: 1470533189000i64});
    tf.push(Tick{price: 23f64, timestamp: 1470533191010i64});
    tf.push(Tick{price: 23.23894f64, timestamp: 1470533192410i64});

    for period in [3, 5].iter() {
        println!("Moving average with period {}", period);

        let mut sma = SMA::new(*period as i64);
        let mut test: Option<f64> = Some(0f64);
        for t in tf.iter() {
            test = sma.push(t);
        }
        println!("{:?}", test);
    }
}
