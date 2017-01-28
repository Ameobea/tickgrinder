//! Strategies are containers for all the logic required to generate
//! trading signals and manage positions for a portfolio.  Only one strategy is
//! meant to run at a time on a single portfolio over which that strategy holds
//! complete control.
//!
//! Strategies are "black boxes" inasmuch as all of their mechanisms for generating
//! trading signals are entirely self-contained.  However, they are configurable
//! manually through the use of `Command`s sent from the MMI or some other source.
//! The main methods through which strategies interact with the world are listed:
//! 1. Commands sent through the optimizer's `CommandServer` to the Tick Processors
//! 2. Direction interaction with the database
//! 3. Live data brodacast over redis channels to which the strategy manually subscribes
//! 4. Ticks inserted into the strategy by the optimizer as they are received live
//!
//! TODO: Update this descrition

use std::collections::HashMap;

use futures::Complete;
use futures::stream::BoxStream;
use uuid::Uuid;

// use trading::broker::Broker;
use transport::command_server::CommandServer;
use transport::query_server::QueryServer;
use trading::objects::BrokerAction;
use trading::broker::Broker;
use trading::tick::Tick;

/// A stream of input ticks from some source (real or artificial) used to drive progress of the strategy.
pub struct Tickstream {
    pub inner: BoxStream<Tick, ()>,
}

/// Wrapper for the interior of a tick processor providing a variety of helper methods and utility functions
/// for management of the handled contingencies and communication with the rest of the platform.
pub struct ContingencyContainer<T> {
    pub cs: CommandServer,
    /// hold which contained contingency handlers are subscribed to which tickstreams
    pub tickstreams: Vec<Tickstream>,
    pub subscriptions: Vec<Vec<usize>>,
    /// The threadsafe part of the container shared by all of the contingency handlers
    pub threadsafe_state: T,
    /// holds the actual contained handlers
    pub handlers: HashMap<Uuid, Box<ContingencyHandler<T>>>,
}

impl<T: Clone + Send> ContingencyContainer<T> {
    pub fn new(state: T, name: &str) -> ContingencyContainer<T> {
        ContingencyContainer {
            cs: CommandServer::new(Uuid::new_v4(), name),
            tickstreams: Vec::new(),
            subscriptions: Vec::new(),
            threadsafe_state: state,
            handlers: HashMap::new(),
        }
    }

    pub fn add_handler(&mut self, handler: Box<ContingencyHandler<T>>, subscriptions: Vec<usize>) -> Uuid {
        self.subscriptions.push(subscriptions);
        let uuid = Uuid::new_v4();
        self.handlers.insert(uuid, handler);
        debug_assert_eq!(self.handlers.len(), self.subscriptions.len());
        uuid
    }

    pub fn get_mut_handler(&mut self, uuid: Uuid) -> Option<&mut Box<ContingencyHandler<T>>> {
        self.handlers.get_mut(&uuid)
    }

    pub fn remove_handler(&mut self, uuid: Uuid) -> Option<Box<ContingencyHandler<T>>> {
        self.handlers.remove(&uuid)
    }
}

/// Wrapper for a user-defined strategy providing a variety of helper methods and utility functions for the
/// individual strategies.  This is passed off to a strategy executor that handles the act of actually ticking
/// the user-defined strategy and driving the process forward.
pub struct StrategyManager<S> {
    pub tickstreams: Vec<Tickstream>,
    pub cs: CommandServer,
    pub qs: QueryServer,
    /// The user-defined portion of the strategy container
    pub strategy: Box<Strategy>,
    pub broker: Box<Broker>,
    /// The user-defined part of the container where persistant state can be stored.
    pub state: S,
}

/// A user-defined piece of logic for controlling the creation, modification, and deletion of
/// `ContingencyHandler`s on the tick parser.
pub trait Strategy {
    /// Instruct the strategy to initialize itself, subscribing to data streams and communicating with the
    /// the rest of the platform as necessary.
    fn init(&mut self);

    // /// Indicates that the strategy should save a copy of its internal state of its internal state to
    // /// the database.  The supplied future should be resolved when the dump is complete.
            // Disabled for now until a time when the platform supports that kind of functionality.
    // fn dump_state(&mut self, done: futures::Complete<()>);

    /// Indicates that the platform is going into an inoperable state and that
    /// the strategy should do whatever necessary to exit as soon as possible.
    /// Provides a oneshot that should be resolved when the strategy is ready to exit.
    fn exit_now(&mut self, ready: Complete<()>);
}

/// Often representing the status of an order or a block of closely related orders, each handler contained in a
/// `ContingencyContainer` is `tick()`ed every time new data is available and any returned actions processed.
pub trait ContingencyHandler<T> {
    /// Called every time new data is available.
    fn tick(&mut self, data_ix: usize, t: &Tick) -> Option<BrokerAction>;

    /// If this is called, it means that we have a situation where the platform can no longer manage this
    /// contingency and we should not expect any further interaction.  Depending on the specifics of the handler,
    /// this action may pull an underlying pending order, simply stop responding to ticks, or take some other action
    /// to deal with the situation.
    fn abort(&mut self);
}

pub trait FSMInnerHandler<S> {
    fn tick(&mut self, old_state: &S, data_ix: usize, t: &Tick) -> (S, Option<BrokerAction>);

    fn abort(&mut self);
}

/// A contingency handler implemented using a Finite State Machine.  Every tick, the handler either keeps its currrent
/// state or transitions to another state.  This is an easy way to define contingency handlers that can deal with
/// multiple different phases of a single task.
///
/// For example, a handler that simply opens an order has to take into account an order not being filled, of being denied
/// for a reason such as insufficient balance, or receiving a partial fill.  With a FSM, it is possible to maintain this
/// full set of actions/responses without having to create new `ContingencyHandler` for each of the possible alternatives.
pub struct FSMContingencyHandler<S> {
    state: S,
    handler: Box<FSMInnerHandler<S>>,
}

impl<S> ContingencyHandler<S> for FSMContingencyHandler<S> {
    fn tick(&mut self, data_ix: usize, t: &Tick) -> Option<BrokerAction> {
        let (state, action_opt) = self.handler.tick(&self.state, data_ix, t);
        self.state = state;
        action_opt
    }

    fn abort(&mut self) {
        self.handler.abort()
    }
}
