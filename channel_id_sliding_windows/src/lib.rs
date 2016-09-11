//! # Channel Identification via Sliding Windows
//!
//! This strategy aims to locate price channels over arbitrary time periods by analyzing the
//! minimums and maximums of prices over various time periods.  By determining new lows of a
//! macro trend and watching for new highs in smaller time periods, subtrends and the channels
//! that they make up can be located and analyzed for trade opportunities.

extern crate futures;
extern crate algobot_util;

use futures::Complete;

use algobot_util::strategies::Strategy;
use algobot_util::tick::SymbolTick;

struct strat{}

impl Strategy for strat {
    fn process(&mut self, t: SymbolTick) {

    }

    fn exit_now(&mut self, ready: Complete<()>) {

    }
}
