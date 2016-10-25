//! Defines a backtest, which determines what data is sent and the
//! conditions that trigger it to be sent.

use std::thread;
use std::time::Duration;

use futures::stream::{Sender, Receiver};
use uuid::Uuid;
use algobot_util::trading::tick::*;

use {BacktestType, DataSource, DataDest};

#[derive(Serialize, Deserialize)]
pub enum BacktestCommand {
    Pause,
    Resume,
    Stop,
}

/// Contains controls for pausing, resuming, and stopping a backtest as well as
/// some data about it.
pub struct BacktestHandle {
    pub uuid: Uuid,
    pub tickstream: Receiver<Tick, ()>,
    pub handle: Sender<BacktestCommand, ()>
}

/// Contains all the information necessary to start a backtest
#[derive(Serialize, Deserialize)]
pub struct BacktestDefinition {
    pub symbol: String,
    pub backtest_type: BacktestType,
    pub data_source: DataSource,
    pub data_dest: DataDest
}

/// Called to determine the timing between ticks sent by the backtest.
pub trait BacktestMap {
    fn map(&mut self, Tick) -> Option<Tick>;
}

/// Inserts a static delay between each tick.
pub struct FastMap {
    pub delay_ms: u64
}

impl BacktestMap for FastMap {
    fn map(&mut self, t: Tick) -> Option<Tick> {
        // block for the delay then return the tick
        thread::sleep(Duration::from_millis(self.delay_ms));
        Some(t)
    }
}

/// Plays ticks back at the rate that they were recorded.
pub struct LiveMap {
    pub last_tick_timestamp: i64
}

impl BacktestMap for LiveMap {
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
