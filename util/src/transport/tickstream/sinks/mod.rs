//! Tick sinks server as the consumers of time series data streams within the platform.  They can be things such as databases where the data
//! is stored, backtests, or strategy executors.

pub mod console_sink;
pub mod csv_sink;
pub mod null_sink;
pub mod redis_sink;
pub mod stream_sink;
