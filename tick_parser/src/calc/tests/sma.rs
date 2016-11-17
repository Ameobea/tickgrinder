use calc::sma::SimpleMovingAverage;
use algobot_util::trading::tick::Tick;

#[test]
fn sma_accuracy() {
    let mut sma = SimpleMovingAverage::new(15);
    let mut t = Tick {bid: 101, ask: 107, timestamp: 1};
    let mut avg = sma.push(t);
    assert_eq!(avg, t.mid());

    t = Tick {bid: 103, ask: 108, timestamp: 5};
    avg = sma.push(t);
    let man_avg = (101 + 107) / 2;
    assert_eq!(avg, man_avg);

    t = Tick {bid: 105, ask: 109, timestamp: 13};
    avg = sma.push(t);
    let man_avg = ((((101 + 107) / 2) * 4) +
                  (((103 + 108) / 2) * 8)) / 12;
    assert_eq!(avg, man_avg);

    t = Tick {bid: 104, ask: 1088, timestamp: 18};
    avg = sma.push(t);
    let man_avg = ((((103 + 108) / 2) * 8) +
                  (((105 + 109) / 2) * 5) +
                  (((101 + 107) / 2) * 2)) / 15;
    assert_eq!(avg, man_avg);
}

fn tick_sma_accuracy() {
    let mut sma = SimpleMovingAverage::new(15);
    let mut t = Tick {bid: 101, ask: 107, timestamp: 1};
    let mut avg_t = sma.push_tick(t);
    assert_eq!(avg_t.mid(), t.mid());

    t = Tick {bid: 103, ask: 108, timestamp: 5};
    avg_t = sma.push_tick(t);
    let man_avg = (101 + 107) / 2;
    assert_eq!(avg_t.mid(), man_avg);

    t = Tick {bid: 105, ask: 109, timestamp: 13};
    avg_t = sma.push_tick(t);
    let man_avg = ((((101 + 107) / 2) * 4) +
                  (((103 + 108) / 2) * 8)) / 12;
    assert_eq!(avg_t.mid(), man_avg);

    t = Tick {bid: 104, ask: 1088, timestamp: 18};
    avg_t = sma.push_tick(t);
    let man_avg = ((((103 + 108) / 2) * 8) +
                  (((105 + 109) / 2) * 5) +
                  (((101 + 107) / 2) * 2)) / 15;
    assert_eq!(avg_t.mid(), man_avg);
}
