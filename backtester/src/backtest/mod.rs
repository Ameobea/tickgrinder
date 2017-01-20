//! Defines a backtest, which determines what data is sent and the
//! conditions that trigger it to be sent.

use std::thread;
use std::time::Duration;
use std::sync::mpsc;
use uuid::Uuid;

use tickgrinder_util::trading::tick::*;

use {BacktestType, DataSource, DataDest};
use simbroker::SimBrokerSettings;

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
    pub start_time: Option<u64>,
    /// Stop backtest after timestamp reached or None
    pub max_timestamp: Option<u64>,
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
    pub last_tick_timestamp: u64
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

/// Ticks sent to the SimBroker should be re-broadcast to the client.
#[test]
fn tick_retransmission() {
    use std::sync::mpsc;
    use std::collections::HashMap;

    use futures::{Future, Stream, oneshot};
    use futures::stream::BoxStream;

    use data::random_reader::RandomReader;
    use data::TickGenerator;
    use backtest::{NullMap, BacktestCommand};
    use simbroker::*;

    // create the SimBroker
    let symbol = "TEST".to_string();
    let mut sim_client = SimBrokerClient::init(HashMap::new()).wait().unwrap().unwrap();
    let msg_stream = sim_client.get_stream();

    // create a random tickstream and register it to the SimBroker
    let mut gen = RandomReader::new(symbol.clone());
    let map = Box::new(NullMap {});
    let (tx, rx) = mpsc::sync_channel(5);
    let tick_stream = gen.get(map, rx);
    // start the random tick generator
    let _ = tx.send(BacktestCommand::Resume);

    // register the tickstream with the simbroker
    let res = sim_client.register_tickstream(symbol.clone(), tick_stream.unwrap(), false, 0);
    assert!(res.is_ok());

    // subscribe to ticks from the SimBroker for the test pair
    let subbed_ticks = sim_client.sub_ticks(symbol).unwrap();

    // start the simbroker's simulation loop
    sim_client.init_sim_loop();

    let (c, o) = oneshot::<Vec<Tick>>();
    thread::spawn(move || {
        let res: Vec<Tick> = subbed_ticks
            .wait()
            .take(10)
            .map(|t| {
                println!("Received tick: {:?}", t);
                t.unwrap()
            })
            .collect();
        // signal once we've received all the ticks
        c.complete(res);
    });

    // block until we've received all awaited ticks
    let res = o.wait().unwrap();
    assert_eq!(res.len(), 10);
}
