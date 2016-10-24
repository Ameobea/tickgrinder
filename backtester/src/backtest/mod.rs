//! Defines a backtest, which determines what data is sent and the
//! conditions that trigger it to be sent.

use futures::stream::{channel, Sender, Receiver};
use uuid::Uuid;

use algobot_util::trading::tick::*;

use data::*;

#[derive(Debug)]
pub enum BacktestType {
    Live,
    Fast,
}

pub enum BacktestCommand {
    Pause,
    Resume,
    Stop,
}

pub struct Backtest {
    pub symbol: String,
    pub backtest_type: BacktestType,
    pub generator_type: &'static str,
    pub tick_stream: Receiver<Tick, ()>
}

// TODO: Simulated broker + Stats
impl Backtest {
    pub fn new<T>(
        symbol: String, backtest_type: BacktestType, mut generator: T
    ) -> Backtest where T:TickGenerator {
        Backtest {
            symbol: symbol.clone(),
            backtest_type: backtest_type,
            generator_type: generator.get_name(),
            tick_stream: generator.get(symbol).expect("Unable to initialize data stream")
        }
    }

    /// Starts a backtest, consuming it, and reading ticks from the generator
    /// and writing them to the Sink.  Returns a BacktestHandle that can be used
    /// to control and monitor the spawned backtest.
    pub fn init<S>(self, endpoint: S) -> BacktestHandle where S:TickSink {
        // TODO
        let (handle_s, handle_r) = channel::<BacktestCommand, ()>();
        BacktestHandle {
            uuid: Uuid::new_v4(),
            symbol: self.symbol,
            backtest_type: self.backtest_type,
            handle: handle_s
        }
    }
}

/// Contains controls for pausing, resuming, and stopping a backtest as well as
/// some data about it.
pub struct BacktestHandle {
    uuid: Uuid,
    symbol: String,
    backtest_type: BacktestType,
    handle: Sender<BacktestCommand, ()>
}
