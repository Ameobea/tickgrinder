//! Defines a backtest, which determines what data is sent and the
//! conditions that trigger it to be sent.

use std::sync::mpsc;
use uuid::Uuid;

use {BacktestType, DataSource, DataDest};
use simbroker::SimBrokerSettings;
use tickgrinder_util::transport::tickstream::TickstreamCommand;

/// Contains controls for pausing, resuming, and stopping a backtest as well as
/// some data about it.
pub struct BacktestHandle {
    pub symbol: String,
    pub backtest_type: BacktestType,
    pub data_source: DataSource,
    pub endpoint: DataDest,
    pub handle: mpsc::SyncSender<TickstreamCommand>,
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

/// Ticks sent to the SimBroker should be re-broadcast to the client.
#[test]
fn tick_retransmission() {
    use std::thread;
    use std::sync::mpsc;
    use std::collections::HashMap;

    use futures::{Future, Stream, oneshot};

    use tickgrinder_util::trading::tick::Tick;
    use simbroker::*;

    // create the SimBroker
    let symbol = "TEST".to_string();
    let mut sim_client = SimBrokerClient::init(HashMap::new()).wait().unwrap().unwrap();

    // subscribe to ticks from the SimBroker for the test pair
    let subbed_ticks = sim_client.sub_ticks(symbol).unwrap();

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
