//! Supplies the backtester with historical ticks stored in a variety of formats.

use futures::stream::Receiver;

use algobot_util::trading::tick::Tick;

use backtest::BacktestMap;

pub mod flatfile_reader;
pub mod redis_reader;
pub mod random_reader;
pub mod redis_sink;

pub use self::flatfile_reader::*;
pub use self::redis_reader::*;
pub use self::random_reader::*;

/// Creates a Stream of Ticks to feed the backtest.
pub trait TickGenerator {
    const NAME: &'static str;

    /// Returns a stream that resolves to new Ticks
    fn get(&mut self, map: &BacktestMap) -> Result<Receiver<Tick, ()>, String>;

    fn get_symbol(&self) -> String;
}

/// Represents an endpoint through which ticks generated in a Backtest can be sent.
///
/// Could be, for example, a Redis channel, IPC bus, database, etc.
pub trait TickSink {
    const NAME: &'static str;

    /// Called every time a new tick is available from the Backtest
    fn tick(&mut self, t: Tick);
}
