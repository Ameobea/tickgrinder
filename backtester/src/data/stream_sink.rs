//! Re-routes the output of the Generator's stream through another stream

use std::thread;
use std::sync::mpsc;

use futures::sync::mpsc::UnboundedSender;
use tickgrinder_util::trading::tick::Tick;

use data::TickSink;

pub struct StreamSink {
    symbol: String,
    mpsc_tx: mpsc::Sender<Tick>,
}

// We map the input stream into a MPSC channel which then sends them through a different
// futures channel due to the fact that futures-rs is terrible but we're WAY too
// commmitted to go back now.
impl StreamSink {
    pub fn new(symbol: String, dst_tx: UnboundedSender<Tick>) -> StreamSink {
        let (tx, rx) = mpsc::channel::<Tick>();
        thread::spawn(move || {
            let dst_tx = dst_tx;
            for t in rx.iter() {
                dst_tx.send(t).expect("Unable to send tick to sink in stream_sink.rs");
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
        self.mpsc_tx.send(t).unwrap();
    }
}
