//! This cleverly named module contains code designed for writing event-level debug data into flatfiles
//! to help verify the simbroker's determinism and weed out issues.

#[cfg(feature = "superlog")]
use futures::{Future, Sink};
#[cfg(feature = "superlog")]
use futures::sync::mpsc::Sender;
use uuid::Uuid;

#[cfg(feature = "superlog")]
use tickgrinder_util::transport::textlog::get_logger_handle;
use tickgrinder_util::trading::objects::Position;

use helpers::CachedPosition;


/// A helper enum to specify which cache action is being taken
#[derive(Debug)]
pub enum CacheAction<'a> {
    OrderPlaced,
    OrderModified{old_order: &'a Position},
    OrderCancelled,
    OrderFilled,
    PositionOpenedImmediate,
    PositionModified{old_pos: &'a Position},
    PositionClosed,
}

// define the versions that actually log for when the `superlog` feature is enabled

#[cfg(feature = "superlog")]
#[derive(Clone)]
pub struct SuperLogger {
    tx: Option<Sender<String>>,
}

#[cfg(feature = "superlog")]
impl SuperLogger {
    pub fn new() -> SuperLogger {
        let tx = get_logger_handle(String::from("simbroker"), 50);

        SuperLogger {
            tx: Some(tx),
        }
    }

    pub fn event_log(&mut self, timestamp: u64, event: &str) {
        let tx = self.tx.take().unwrap();
        let log_line = format!("{}: {}", timestamp, event);
        // println!("SIMBROKER: {}", log_line);
        let new_tx = tx.send(log_line).wait().unwrap();
        self.tx = Some(new_tx);
    }

    /// Log a cache event.
    pub fn cache_log(&mut self, action: CacheAction, account_uuid: Uuid, pos_uuid: Uuid, pos: &Position) {
        let tx = self.tx.take().unwrap();
        let log_line = format!("CACHE - {:?}, AccountID: {}, PosID: {}, pos: {:?}", action, account_uuid, pos_uuid, pos);
        let new_tx = tx.send(log_line).wait().unwrap();
        self.tx = Some(new_tx);
    }

    pub fn error_log(&mut self, err_msg: &str) {
        let tx = self.tx.take().unwrap();
        let log_line = format!("ERROR - {}", err_msg);
        let new_tx = tx.send(log_line).wait().unwrap();
        self.tx = Some(new_tx);
    }
}

// and now the dummy versions that should optmize into oblivion is `superlog` is disabled

#[cfg(not(feature = "superlog"))]
#[derive(Clone)]
pub struct SuperLogger {}

#[cfg(not(feature = "superlog"))]
impl SuperLogger {
    pub fn new() -> SuperLogger {
        SuperLogger {}
    }

    #[allow(unused_variables)]
    pub fn event_log(&mut self, timestamp: u64, event: &str) {
        // do nothing; this should optimize out completely.
    }

    #[allow(unused_variables)]
    pub fn cache_log(&mut self, action: CacheAction, account_uuid: Uuid, pos_uuid: Uuid, pos: &Position) {
        // do nothing; this should optimize out completely.
    }

    #[allow(unused_variables)]
    pub fn error_log(&mut self, err_msg: &str) {
        // do nothing; this should optimize out completely.
    }
}
