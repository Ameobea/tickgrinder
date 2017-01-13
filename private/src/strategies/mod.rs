//! Strategies are the core pieces of log that tell the platform how to process data.

use std::collections::HashMap;

use tickgrinder_util::strategies::Strategy;

mod sma_cross;

// Set this to whichever strategy you want to use.
pub use self::sma_cross::SmaCross as ActiveStrategy;

// Returns K:V settings to be sent to the broker during initialization
pub fn get_broker_settings() -> HashMap<String, String> {
    HashMap::new() // TODO
}
