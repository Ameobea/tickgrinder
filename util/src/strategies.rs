//! Defines the basis of the strategy system for the platform.  The main way of defining strategies has two parts:
//! The `Strategy` which accepts data from all parts of the platform and is in charge of managing the
//! `TradingCondition`s on the tick processor and the `TradingConditon`s themselves which are the lowest level of
//! independent control that the platform supports.  The state of each handler is only known to that handler and
//! they can only communicate events back to their parent strategy via `ContingencyAction`s.
//!
//! Strategies are not responsible for driving their own progress or requesting/querying for data from external
//! sources or tickstreams.  Instead, a strategy executor manages this externally leaving strategies only the task
//! of processing the supplied data.
//!
//! TODO: Write readme file on the whole strategy system once it's (more) finalized

use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use uuid::Uuid;

// use trading::broker::Broker;
use transport::command_server::CommandServer;
use transport::query_server::QueryServer;
use transport::commands::Command;
use trading::objects::BrokerAction;
use trading::broker::{Broker, BrokerResult};
use trading::tick::{Tick, GenTick};

/// Holds metadata about a tickstream.
pub struct Tickstream {
    name: String,
    is_fx: bool,
    decimal_precision: usize,
}

/// Wrapper for the interior of a tick processor providing a variety of helper methods and utility functions
/// for management of the handled contingencies and communication with the rest of the platform.
pub struct ContingencyHandlerManager {
    pub helper: Helper,
    /// Holds the signatures and raw `Stream`s of all subscribed tickstreams.
    pub tickstreams: Vec<Tickstream>,
    /// hold which contained contingency handlers are subscribed to which tickstreams
    pub subscriptions: Vec<Vec<usize>>,
    /// holds the actual contained handlers.  These are evaluated in order every tick.
    pub handlers: Handlers,
    /// The executor for events returned by the `ContingencyHandler`s
    pub executor: Box<ContingencyHandlerEventExecutor>,
}

pub trait ContingencyHandlerEventExecutor {
    fn exec(&mut self, event: &ContingencyAction);
}

pub struct Handlers {
    pub hm: HashMap<Uuid, usize>,
    pub data: Vec<Box<ContingencyHandler>>,
}

impl Handlers {
    pub fn new() -> Handlers {
        Handlers {
            hm: HashMap::new(),
            data: Vec::new(),
        }
    }

    pub fn insert(&mut self, uuid: Uuid, handler: Box<ContingencyHandler>) {
        self.hm.insert(uuid, self.data.len());
        self.data.push(handler);
    }

    pub fn remove(&mut self, uuid: Uuid) -> Box<ContingencyHandler> {
        let ix = self.hm.remove(&uuid).unwrap();
        for (_, v) in self.hm.iter_mut() {
            // decrement the index of all the others since we've removed an element from the vector.
            if *v > ix {
                *v -= 1;
            }
        }
        self.data.remove(ix)
    }

    pub fn get_mut(&mut self, uuid: Uuid) -> &mut Box<ContingencyHandler> {
        let ix = self.hm.get(&uuid).unwrap();
        &mut self.data[*ix]
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

/// Allow immutable indexing of the inner data Vector for high-speed internal data access
impl Index<usize> for Handlers {
    type Output = Box<ContingencyHandler>;

    fn index(&self, index: usize) -> &Self::Output {
        self.data.get(index).unwrap()
    }
}

/// Allow mutable indexing of the inner data Vector for high-speed internal data access
impl IndexMut<usize> for Handlers {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.data.get_mut(index).unwrap()
    }
}

/// Allow immutable indexing of the inner data `Vec` by `String` via looking up through the internal `HashMap`.
impl<'a> Index<&'a Uuid> for Handlers {
    type Output = Box<ContingencyHandler>;

    fn index(&self, index: &'a Uuid) -> &Self::Output {
        match self.hm.get(index) {
            Some(ix) => self.data.get(*ix).unwrap(),
            None => panic!("Attempted to get {} by Uuid but can't find a match!", index),
        }
    }
}

/// Allow mutable indexing of the inner data `Vec` by `String` via looking up through the internal `HashMap`.
impl<'a> IndexMut<&'a Uuid> for Handlers {
    fn index_mut(&mut self, index: &'a Uuid) -> &mut Self::Output {
        match self.hm.get(index) {
            Some(ix) => self.data.get_mut(*ix).unwrap(),
            None => panic!("Attempted to get {} by Uuid but can't find a match!", index),
        }
    }
}

impl ContingencyHandlerManager {
    pub fn new(executor: Box<ContingencyHandlerEventExecutor>, broker: Box<Broker>) -> ContingencyHandlerManager {
        ContingencyHandlerManager {
            helper: Helper::new(broker),
            tickstreams: Vec::new(),
            subscriptions: Vec::new(),
            handlers: Handlers::new(),
            executor: executor,
        }
    }

    /// Called when new data is available.  This passes on the message to all subscribed `ContingencyHandler`s
    /// and executes all returned events using the `ContingencyHandlerEventExecutor`.
    pub fn tick(&mut self, data_ix: usize, t: &Tick) {
        for ix in &self.subscriptions[data_ix] {
            let actions = self.handlers[*ix].tick(data_ix, t);
            for action in actions {
                self.executor.exec(action);
            }
        }
    }

    pub fn add_handler(&mut self, handler: Box<ContingencyHandler>, subscriptions: Vec<usize>) -> Uuid {
        self.subscriptions.push(subscriptions);
        let uuid = Uuid::new_v4();
        self.handlers.insert(uuid, handler);
        debug_assert_eq!(self.handlers.len(), self.subscriptions.len());
        uuid
    }

    pub fn get_mut_handler(&mut self, uuid: Uuid) -> &mut Box<ContingencyHandler> {
        self.handlers.get_mut(uuid)
    }

    pub fn remove_handler(&mut self, uuid: Uuid) -> Box<ContingencyHandler> {
        self.handlers.remove(uuid)
    }
}

/// Often representing the status of an order or a block of closely related orders, each handler contained in a
/// `ContingencyContainer` is `tick()`ed every time new data is available and any returned actions processed.
pub trait ContingencyHandler {
    /// Called every time new data is available.  Provides the threadsafe state
    fn tick(&mut self, data_ix: usize, t: &Tick) -> &[ContingencyAction];

    /// If this is called, it means that we have a situation where the platform can no longer manage this
    /// contingency and we should not expect any further interaction.  Depending on the specifics of the handler,
    /// this action may pull an underlying pending order, simply stop responding to ticks, or take some other action
    /// to deal with the situation.
    fn abort(&mut self);

    /// Returns the index of all tickstreams from the parent `ContingencyHandlerManager`
    fn get_subscriptions(&self) -> Vec<usize>;
}

pub struct Helper {
    pub cs: CommandServer,
    pub qs: QueryServer,
    pub broker: Box<Broker>,
}

impl Helper {
    pub fn new(broker: Box<Broker>) -> Helper {
        Helper {
            cs: CommandServer::new(Uuid::new_v4(), "Strategy Helper Utility"),
            qs: QueryServer::new(4),
            broker: broker,
        }
    }
}

/// A type representing the data connected to a Tick provided by a `ManagedStrategy`.  Contains pre-defined
/// variants for data received from the broker as well as a slot for user-defined data types.
pub enum Merged<T> {
    BrokerPushstream(BrokerResult),
    BrokerTick(usize, Tick),
    T(T),
}

/// Wrapper for a user-defined strategy providing a variety of helper methods and utility functions for the
/// individual strategies.  This is passed off to a strategy executor that handles the act of actually ticking
/// the user-defined strategy and driving the process forward.
pub struct StrategyManager<T> {
    pub helper: Helper,
    pub subscriptions: Vec<Tickstream>,
    /// The user-defined portion of the strategy container
    pub strategy: Box<ManagedStrategy<T>>,
    pub tickstream_definitions: Vec<Tickstream>,
}

impl<T> StrategyManager<T> {
    pub fn new(
        strategy: Box<ManagedStrategy<T>>, broker: Box<Broker>, tickstream_definitions: Vec<Tickstream>
    ) -> StrategyManager<T> {
        StrategyManager {
            helper: Helper::new(broker),
            subscriptions: Vec::new(),
            strategy: strategy,
            tickstream_definitions: tickstream_definitions,
        }
    }

    pub fn t_tick(&mut self, t: GenTick<T>) -> Option<StrategyAction> {
        self.strategy.tick(&mut self.helper, &GenTick{timestamp: t.timestamp, data: Merged::T(t.data)})
    }

    pub fn broker_tick(&mut self, data_ix: usize, t: Tick) -> Option<StrategyAction> {
        self.strategy.tick(&mut self.helper, &GenTick{timestamp: t.timestamp, data: Merged::BrokerTick(data_ix, t)})
    }

    pub fn pushstream_tick(&mut self, msg: BrokerResult, timestamp: u64) -> Option<StrategyAction> {
        self.strategy.tick(&mut self.helper, &GenTick{timestamp: timestamp, data: Merged::BrokerPushstream(msg)})
    }
}

/// A strategy managed by a `StrategyManager`.  Every call to `tick()` contains a reference to the utility
/// structure that supplies helper functions to the CommandServer, QueryServer, and other miscllanious
/// platform utilities.
pub trait ManagedStrategy<T> {
    fn init(&mut self, helper: &mut Helper, subscriptions: &[Tickstream]);

    fn tick(&mut self, helper: &mut Helper, t: &GenTick<Merged<T>>) -> Option<StrategyAction>;

    fn abort(&mut self);
}

impl<T> Strategy<Merged<T>> for StrategyManager<T> {
    fn init(&mut self) {
        self.strategy.init(&mut self.helper, self.subscriptions.as_slice())
    }

    fn tick(&mut self, t: &GenTick<Merged<T>>) -> Option<StrategyAction> {
        self.strategy.tick(&mut self.helper, t)
    }

    fn abort(&mut self) {
        self.strategy.abort()
    }
}

/// Contains all possible actions that a strategy can take.
pub enum StrategyAction {
    AddContingencyHandler(Box<ContingencyHandler>),
    RemoveContingencyManager(Uuid),
    BrokerAction(BrokerAction),
    PlatformCommand(Command),
}

pub enum ContingencyAction {
    BrokerAction(BrokerAction),
    /// A message to be sent back to the parent strategy communicating some event or change in state that
    /// has occured.
    Message(String),
}

/// A user-defined piece of logic for controlling the creation, modification, and deletion of
/// `ContingencyHandler`s on the tick parser.
pub trait Strategy<T> {
    /// Instruct the strategy to initialize itself, subscribing to data streams and communicating with the
    /// the rest of the platform as necessary.
    fn init(&mut self);

    /// Every time the strategy receives data
    fn tick(&mut self, t: &GenTick<T>) -> Option<StrategyAction>;

    // /// Indicates that the strategy should save a copy of its internal state of its internal state to
    // /// the database.  The supplied future should be resolved when the dump is complete.
            // Disabled for now until a time when the platform supports that kind of functionality.
    // fn dump_state(&mut self, done: futures::Complete<()>);

    /// Indicates that the platform is going into an unsustainable state and that the strategy should do
    /// whatever is necessary to exit as soon as possible.
    fn abort(&mut self);
}

pub trait FSMInnerHandler<S> {
    fn tick(&mut self, old_state: &S, data_ix: usize, t: &Tick) -> (S, &[ContingencyAction]);

    fn abort(&mut self);

    fn get_subscriptions(&self) -> Vec<usize>;
}

/// A contingency handler implemented using a Finite State Machine.  Every tick, the handler either keeps its currrent
/// state or transitions to another state.  This is an easy way to define contingency handlers that can deal with
/// multiple different phases of a single task.  `S` represents the state of the handler.
///
/// For example, a handler that simply opens an order has to take into account an order not being filled, of being denied
/// for a reason such as insufficient balance, or receiving a partial fill.  With a FSM, it is possible to maintain this
/// full set of actions/responses without having to create new `ContingencyHandler` for each of the possible alternatives.
pub struct FSMContingencyHandler<S> {
    pubstate: S,
    handler: Box<FSMInnerHandler<S>>,
}

impl<S> ContingencyHandler for FSMContingencyHandler<S> {
    fn tick(&mut self, data_ix: usize, t: &Tick) -> &[ContingencyAction] {
        let (new_state, action_opt) = self.handler.tick(&mut self.pubstate, data_ix, t);
        self.pubstate = new_state;
        action_opt
    }

    fn abort(&mut self) {
        self.handler.abort()
    }

    fn get_subscriptions(&self) -> Vec<usize> {
        self.handler.get_subscriptions()
    }
}
