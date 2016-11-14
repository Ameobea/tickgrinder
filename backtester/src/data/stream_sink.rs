//! Re-routes the output of the Generator's stream through another stream

use std::thread;
use std::sync::mpsc;

use futures::Future;
use futures::stream::Sender;
use algobot_util::trading::tick::Tick;

use data::TickSink;

pub struct StreamSink {
    symbol: String,
    mpsc_tx: mpsc::Sender<Tick>,
}

// We map the input stream into a MPSC channel which then sends them through a different
// futures channel due to the fact that futures-rs is terrible but we're WAY too
// commmitted to go back now.
impl StreamSink {
    pub fn new(symbol: String, dst_tx: Sender<Tick, ()>) -> StreamSink {
        let (tx, rx) = mpsc::channel::<Tick>();
        thread::spawn(move || {
            let mut dst_tx = dst_tx;
            for t in rx.iter() {
                dst_tx = dst_tx.send(Ok(t)).wait().ok().unwrap();
            }
        });

        StreamSink {
            symbol: symbol,
            mpsc_tx: tx,
        }
    }
}

impl TickSink for StreamSink {
    fn tick(&mut self, t: Tick) {
        let _ = self.mpsc_tx.send(t);
    }
}
