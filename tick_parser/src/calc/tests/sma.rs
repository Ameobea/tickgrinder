use calc::sma::SimpleMovingAverage;
use algobot_util::trading::tick::Tick;

#[test]
fn sma_accuracy() {
    let mut sma = SimpleMovingAverage::new(15f64);
    let mut t = Tick {bid: 1.01, ask: 1.07, timestamp: 1};
    let mut avg = sma.push(t);
    assert_eq!(avg, t.mid());

    t = Tick {bid: 1.03, ask: 1.08, timestamp: 5};
    avg = sma.push(t);
    let man_avg = (1.01f64 + 1.07f64) / 2f64;
    assert_eq!(avg, man_avg);

    t = Tick {bid: 1.05, ask: 1.09, timestamp: 13};
    avg = sma.push(t);
    let man_avg = ((((1.01 + 1.07) / 2f64) * 4f64) +
                  (((1.03 + 1.08) / 2f64) * 8f64)) / 12f64;
    assert_eq!(avg, man_avg);

    t = Tick {bid: 1.04, ask: 1.088, timestamp: 18};
    avg = sma.push(t);
    let man_avg = ((((1.03 + 1.08) / 2f64) * 8f64) +
                  (((1.05 + 1.09) / 2f64) * 5f64) +
                  (((1.01 + 1.07) / 2f64) * 2f64)) / 15f64;
    assert_eq!(avg, man_avg);
}

fn tick_sma_accuracy() {
    let mut sma = SimpleMovingAverage::new(15f64);
    let mut t = Tick {bid: 1.01, ask: 1.07, timestamp: 1};
    let mut avg_t = sma.push_tick(t);
    assert_eq!(avg_t.mid(), t.mid());

    t = Tick {bid: 1.03, ask: 1.08, timestamp: 5};
    avg_t = sma.push_tick(t);
    let man_avg = (1.01f64 + 1.07f64) / 2f64;
    assert_eq!(avg_t.mid(), man_avg);

    t = Tick {bid: 1.05, ask: 1.09, timestamp: 13};
    avg_t = sma.push_tick(t);
    let man_avg = ((((1.01 + 1.07) / 2f64) * 4f64) +
                  (((1.03 + 1.08) / 2f64) * 8f64)) / 12f64;
    assert_eq!(avg_t.mid(), man_avg);

    t = Tick {bid: 1.04, ask: 1.088, timestamp: 18};
    avg_t = sma.push_tick(t);
    let man_avg = ((((1.03 + 1.08) / 2f64) * 8f64) +
                  (((1.05 + 1.09) / 2f64) * 5f64) +
                  (((1.01 + 1.07) / 2f64) * 2f64)) / 15f64;
    assert_eq!(avg_t.mid(), man_avg);
}
