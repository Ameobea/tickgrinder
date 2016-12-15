//! Sample Strategy demonstrating usage of the Strategy trait.

#![feature(conservative_impl_trait)]
#![allow(unused_imports, dead_code, unused_variables)]

extern crate futures;
extern crate algobot_util;

use std::thread;

use futures::Complete;

use algobot_util::strategies::Strategy;
use algobot_util::trading::tick::SymbolTick;
use algobot_util::trading::broker::Broker;
use algobot_util::transport::command_server::CommandServer;
use algobot_util::transport::query_server::QueryServer;

pub struct SampleStrategy<'b> {
    cs: CommandServer,
    qs: QueryServer,
    broker: &'b mut Broker,
}

/// This function is called by the Optimizer in order to create this strategy.  You must include
/// it and it just call your strategy's `new()` method.
pub fn new<'a, B>(cs: CommandServer, qs: QueryServer, broker: &'a mut B) -> impl Strategy<'a> where B:Broker {
    SampleStrategy::new(cs, qs, broker)
}

impl<'a> Strategy<'a> for SampleStrategy<'a> {
    /// This should initialize the strategy and start it running.  It should be asynchronous so
    /// the strategy doesn't block the main thread, so it's likely that the strategy should run
    fn new<'b, B>(cs: CommandServer, qs: QueryServer, broker: &'a mut B) -> Self where B:Broker + 'b + 'a {
        SampleStrategy {
            cs: cs,
            qs: qs,
            broker: broker,
        }
    }

    /// Instructs the strategy to initialize itself, subscribing to data streams and communicating with the
    /// the rest of the platform as necessary
    fn init(&mut self) {
        // get a stream of live ticks for EURUSD from the broker
        let rx = self.broker.sub_ticks("EURUSD".to_string());
    }

    /// This should return a String containing all of (or as much as possible of) the strategy's
    /// internal state.  If enabled, it will be called periodically by the optimizer to keep
    /// backups of the Strategy to allow quick restoration in case of a crash or similar event.
    fn dump_state(&mut self, done: futures::Complete<()>) {
        unimplemented!();
    }

    /// If this is called, it indicates that the optimizer/platform is going down imminently and that
    /// the strategy should do anything it needs to do before that happens.
    fn exit_now(&mut self, ready: Complete<()>) {
        unimplemented!();
    }
}
