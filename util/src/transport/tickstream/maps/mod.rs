//! Contains all the `TickMap`s for the platform.

use super::*;

pub mod poloniex;

pub use self::poloniex::*;

/// Inserts a static delay between each tick.
pub struct FastMap {
    pub delay_ms: usize
}

impl TickMap for FastMap {
    fn map(&mut self, t: Tick) -> Option<Tick> {
        // block for the delay then return the tick
        thread::sleep(Duration::from_millis(self.delay_ms as u64));
        Some(t)
    }
}

/// Plays ticks back at the rate that they were recorded.
pub struct LiveMap {
    pub last_tick_timestamp: u64
}

impl TickMap for LiveMap {
    fn map(&mut self, t: Tick) -> Option<Tick> {
        if self.last_tick_timestamp == 0 {
            self.last_tick_timestamp = t.timestamp;
            return None
        }

        let diff = t.timestamp - self.last_tick_timestamp;
        thread::sleep(Duration::from_millis(diff as u64));
        Some(t)
    }
}

impl LiveMap {
    pub fn new() -> LiveMap {
        LiveMap {
            last_tick_timestamp: 0
        }
    }
}

pub struct NullMap {}

impl TickMap for NullMap {
    fn map(&mut self, t: Tick) -> Option<Tick> { Some(t) }
}
