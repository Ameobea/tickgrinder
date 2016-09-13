//! # Channel Identification via Sliding Windows
//!
//! This strategy aims to locate price channels over arbitrary time periods by analyzing the
//! minimums and maximums of prices over various time periods.  By determining new lows of a
//! macro trend and watching for new highs in smaller time periods, subtrends and the channels
//! that they make up can be located and analyzed for trade opportunities.

#![feature(test, conservative_impl_trait)]

extern crate futures;
extern crate algobot_util;
extern crate test;

mod window_manager;

use futures::Complete;
use algobot_util::strategies::Strategy;
use algobot_util::tick::SymbolTick;
use algobot_util::transport::command_server::CommandServer;
use algobot_util::transport::query_server::QueryServer;

use window_manager::WindowManager;

struct SlidingWindows {
    wm: WindowManager
}

impl SlidingWindows {
    fn new() -> SlidingWindows {
        SlidingWindows {
            wm: WindowManager::new()
        }
    }
}

impl Strategy for SlidingWindows {
    /// This should initialize the strategy and start it running.  It should be asynchronous so
    /// the strategy doesn't block the main thread, so it's likely that the strategy should run
    /// on a separate thread.
    fn init(cs: CommandServer, qs: QueryServer) {
        let strat = SlidingWindows::new();
    }

    fn process(&mut self, t: SymbolTick) {

    }

    fn exit_now(&mut self, ready: Complete<()>) {

    }
}
