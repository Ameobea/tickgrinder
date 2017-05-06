//! Defines a backtest, which determines what data is sent and the
//! conditions that trigger it to be sent.

use std::thread;
use std::time::Duration;
use std::sync::mpsc;
use uuid::Uuid;

use algobot_util::trading::tick::*;

use {BacktestType, DataSource, DataDest};
use sim_broker::SimBrokerSettings;

/// Commands for controlling a backtest
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BacktestCommand {
    Pause,
    Resume,
    Stop,
}

/// Contains controls for pausing, resuming, and stopping a backtest as well as
/// some data about it.
pub struct BacktestHandle {
    pub symbol: String,
    pub backtest_type: BacktestType,
    pub data_source: DataSource,
    pub endpoint: DataDest,
    pub handle: mpsc::SyncSender<BacktestCommand>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableBacktestHandle {
    pub uuid: Uuid,
    pub symbol: String,
    pub backtest_type: BacktestType,
    pub data_source: DataSource,
    pub endpoint: DataDest,
}

impl SerializableBacktestHandle {
    pub fn from_handle(handle: &BacktestHandle, uuid: Uuid) -> SerializableBacktestHandle {
        SerializableBacktestHandle {
            uuid: uuid,
            symbol: handle.symbol.clone(),
            backtest_type: handle.backtest_type.clone(),
            data_source: handle.data_source.clone(),
            endpoint: handle.endpoint.clone(),
        }
    }
}

/// Contains all the information necessary to start a backtest
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BacktestDefinition {
    pub start_time: Option<usize>,
    /// Stop backtest after timestamp reached or None
    pub max_timestamp: Option<usize>,
    /// Stop backtest after `max_tick_n` ticks have been processed or None
    pub max_tick_n: Option<usize>,
    pub symbol: String,
    pub backtest_type: BacktestType,
    pub data_source: DataSource,
    pub data_dest: DataDest,
    pub broker_settings: SimBrokerSettings,
}

/// Called to determine the timing between ticks sent by the backtest.
pub trait BacktestMap {
    fn map(&mut self, Tick) -> Option<Tick>;
}

/// Inserts a static delay between each tick.
pub struct FastMap {
    pub delay_ms: usize
}

impl BacktestMap for FastMap {
    fn map(&mut self, t: Tick) -> Option<Tick> {
        // block for the delay then return the tick
        thread::sleep(Duration::from_millis(self.delay_ms as u64));
        Some(t)
    }
}

/// Plays ticks back at the rate that they were recorded.
pub struct LiveMap {
    pub last_tick_timestamp: usize
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

pub struct NullMap {}

impl BacktestMap for NullMap {
    fn map(&mut self, t: Tick) -> Option<Tick> { Some(t) }
}
