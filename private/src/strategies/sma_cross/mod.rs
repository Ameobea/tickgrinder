//! A basic "hello world" strategy to show the platform's functionality.  The strategy places buy orders when
//! the price crosses over the SMA and places sell orders when it crosses back.
//!
//! See /util/src/strategies.rs for more detailed documentation on how to implement the Strategy trait.

use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use std::fmt::Debug;

use futures::{Future, Complete};

use tickgrinder_util::strategies::{StrategyManager, ManagedStrategy, Helper, StrategyAction, Tickstream};
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::query_server::QueryServer;
use tickgrinder_util::trading::broker::Broker;
use tickgrinder_util::trading::tick::Tick;

use ActiveBroker;
use super::get_broker_settings;

#[derive(Clone)]
pub struct SmaCross {}

impl ManagedStrategy for SmaCross {
    /// Called when we are to start actively trading this strategy and initialize trading activity.
    fn init(&mut self, helper: &mut Helper, subscriptions: &[Tickstream]) {
        helper.cs.notice(Some("Startup"), "SMA Cross strategy is being initialized...");
            let accounts = unwrap_log_panic(helper.broker.list_accounts().wait().unwrap(), &mut helper.cs);
            let accounts_dbg = format!("Accounts on the broker: {:?}", accounts);
            println!("{}", accounts_dbg);
    }

    fn tick(&mut self, helper: &mut Helper, data_ix: usize, t: &Tick) -> Option<StrategyAction> {
        unimplemented!();
    }

    /// Indicates that the platform is shutting down and that we need to do anything necessary (closing positions)
    /// before that happens.  Includes a future to complete once we're ready.
    fn abort(&mut self) {}
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
