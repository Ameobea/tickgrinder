//! This cleverly named module contains code designed for writing event-level debug data into flatfiles
//! to help verify the simbroker's determinism and weed out issues.

#[cfg(feature = "superlog")]
    use futures::{Future, Sink};
#[cfg(feature = "superlog")]
use futures::sync::mpsc::Sender;

#[cfg(feature = "superlog")]
use tickgrinder_util::transport::textlog::get_logger_handle;

// define the versions that actually log for when the `superlog` feature is enabled

#[cfg(feature = "superlog")]
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
        println!("SIMBROKER: {}", log_line);
        let new_tx = tx.send(log_line).wait().unwrap();
        self.tx = Some(new_tx);
    }
}

// and now the dummy versions that should optmize into oblivion is `superlog` is disabled

#[cfg(not(feature = "superlog"))]
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
}
