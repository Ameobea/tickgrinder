//! A basic "hello world" strategy to show the platform's functionality.  The strategy places buy orders when
//! the price crosses over the SMA and places sell orders when it crosses back.
//!
//! See /util/src/strategies.rs for more detailed documentation on how to implement the Strategy trait.

use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use std::fmt::Debug;

use futures::{Future, Complete};

use algobot_util::strategies::Strategy;
use algobot_util::transport::command_server::CommandServer;
use algobot_util::transport::query_server::QueryServer;
use algobot_util::trading::broker::Broker;

use ActiveBroker;
use super::get_broker_settings;

#[derive(Clone)]
pub struct SmaCross {
    pub cs: CommandServer,
    pub qs: QueryServer,
}

impl Strategy for SmaCross {
    fn new(cs: CommandServer, qs: QueryServer) -> SmaCross {
        SmaCross {
            cs: cs,
            qs: qs,
        }
    }

    /// Called when we are to start actively trading this strategy and initialize trading activity.
    fn init(&mut self) {
        self.cs.notice(Some("Startup"), "SMA Cross strategy is being initialized...");

        let strat_clone = self.clone();
        thread::spawn(move || {
            init_strat(strat_clone, get_broker_settings());
        });
    }

    /// Indicates that the platform is shutting down and that we need to do anything necessary (closing positions)
    /// before that happens.  Includes a future to complete once we're ready.
    fn exit_now(&mut self, ready: Complete<()>) {
        // complete the future to indicate that we're ready to be shut down
        ready.complete(());
    }
}

/// The inner logic for this strategy.  Called once the strategy is initialized.  It will block indefinately as long as
/// the strategy remains active.
fn init_strat(strat: SmaCross, settings: HashMap<String, String>) {
    let mut cs = strat.cs;
    cs.notice(Some("Startup"), "Creating connection to broker...");
    let broker_res = ActiveBroker::init(settings).wait().unwrap();
    let mut broker = unwrap_log_panic(broker_res, &mut cs);
    cs.notice(Some("Startup"), "Successfully connected to broker.");

    let accounts = unwrap_log_panic(broker.list_accounts().wait().unwrap(), &mut cs);
    let accounts_dbg = format!("Accounts on the broker: {:?}", accounts);
    println!("{}", accounts_dbg);
}

/// Unwraps a Result and returns the inner value if it is Ok; `panic!()`s after logging a critical error otherwise.
fn unwrap_log_panic<T, E>(res: Result<T, E>, cs: &mut CommandServer) -> T where E:Debug {
    match res {
        Ok(val) => val,
        Err(err) => {
            let err_string = format!("{:?}", err);
            cs.critical(None, &format!("There was an error when unwrapping Result: {}", err_string));
            // make sure that the message actually gets sent before dying
            thread::sleep(Duration::from_secs(1));
            panic!(err_string);
        }
    }
}
