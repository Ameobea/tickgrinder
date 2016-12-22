//! Indicators transform data from a live data source, database, or other indicator into some
//! metric that can be used to create trading signals or analyze data.

use std::collections::HashMap;

use trading::tick::*;

/// This trait is used to use an indicator to read historical data from the database or some
/// other source and return in in a format that the Monitor plotting API understands (JSON).
pub trait HistQuery {
    /// Returns a JSON-formatted string containing an Array of timestamped indicator values
    /// through the specified time range.  Args is any additional indicator-specific
    /// arguments that need to be given.  Period is the minimum time that must elapse between
    /// two distinct indicator values returned.
    fn get(start_time: usize, end_time: usize, period: usize, args: HashMap<String, String>) -> Result<String, String>;
}

/// Implemented for indicators that can process live ticks.  The `tick()` function should be
/// called for every live tick and an arbitrary data type returned.
pub trait LiveQuery<T> {
    /// Called every new tick received; optionally returns an arbitrary piece of data every time
    /// a new one is received.
    fn tick(t: Tick) -> Option<T>;
}
