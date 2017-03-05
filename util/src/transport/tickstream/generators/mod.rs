//! Tick generators take data from an external source and create a stream of time series data out of it that can be saved to storage, used
//! for a backtest, or fed into strategies during a live trading system.

pub mod flatfile_reader;
pub mod postgres_reader;
pub mod random_reader;
pub mod redis_reader;
