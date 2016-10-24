//! Supplies the backtester with historical ticks stored in a variety of formats.

use futures::stream::Receiver;

use algobot_util::trading::tick::Tick;

pub mod flatfile_reader;
pub mod redis_reader;
pub mod random_reader;

/// Creates a Stream of Ticks to feed the backtest that originate from some source.
pub trait TickGenerator {
    /// Returns a stream that resolves to new Ticks
    fn get(&mut self, symbol: String) -> Result<Receiver<Tick, ()>, String>;

    /// Returns a &str telling what kind of generator it is (flatfile, random, etc.)
    fn get_name(&self) -> &'static str;
}

/// Represents an endpoint through which ticks generated in a Backtest can be sent.
///
/// Could be, for example, a Redis channel, IPC bus, database, etc.
pub trait TickSink {
    /// Called every time a new tick is available from the Backtest
    fn tick(t: Tick);

    /// Returns a &str telling what kind of sink it is (Redis, File, DB, etc.)
    fn get_name(&mut self) -> &'static str;
}
