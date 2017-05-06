//! Strategies are containers for all the logic required to generate
//! trading signals and manage positions for a portfolio.  Only one strategy is
//! meant to run at a time on a single portfolio over which that strategy holds
//! complete control.
//!
//! Strategies are "black boxes" inasmuch as all of their mechanisms for generating
//! trading signals are entirely self-contained.  However, they are configurable
//! manually through the use of Commands sent from the MMI or some other source.
//! The main methods through which strategies interact with the world are listed:
//! 1. Commands sent through the optimizer's CommandServer to the Tick Processors
//! 2. Direction interaction with the database
//! 3. Live data brodacast over redis channels to which the strategy manually subscribes
//! 4. Ticks inserted into the strategy by the optimizer as they are received live

use futures;

use trading::broker::Broker;
use transport::command_server::CommandServer;
use transport::query_server::QueryServer;

pub trait Strategy<'a> {
    /// Make sure that all strategies include ways to interact with the optimizer in a standardized way
    fn new<'b, B>(cs: CommandServer, qs: QueryServer, broker: &'a mut B) -> Self where B:Broker + 'b;

    /// Instruct the strategy to initialize itself, subscribing to data streams and communicating with the
    /// the rest of the platform as necessary
    fn init(&mut self);

    /// Indicates that the strategy should save a copy of its internal state of its internal state to
    /// the database.  The supplied future should be resolved when the dump is complete.
    fn dump_state(&mut self, done: futures::Complete<()>);

    /// Indicates that the platform is going into an inoperable state and that
    /// the strategy should do whatever necessary to exit as soon as possible.
    /// Provides a oneshot that should be resolved when the strategy is ready to exit.
    fn exit_now(&mut self, ready: futures::Complete<()>);
}
