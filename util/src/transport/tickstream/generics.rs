//! Contains the definitions of generic tick processing objects.
//!
//! The general idea is that ticks are created or pulled from some external source in the `GenTickGenerator`s,
//! are processed internally by the `GenTickMap`s possibly returning a different type of tick, and finally
//! consumed by the `GenTickSink`s.

// This is currently experimental (more so than the rest of the platform) and is not a certain implementation.

// TODO: Organize the individual sinks, gens, maps into subdirectories

use std::collections::HashMap;

use futures::sync::mpsc::Receiver;
use serde::{Serialize, Deserialize};

use trading::tick::GenTick;
use transport::command_server::CommandServer;

/// Represents an external source of data that can be used to produce ticks.
pub trait GenTickGenerator<T> {
    /// Given some settings in the form of a `String`:`String` `HashMap`, returns a new instance of a generator
    fn new(settings: HashMap<String, String>) -> Result<Self, String> where Self:Sized;

    /// Retruns a `Stream` of `GenTick<T>`s from the generator.
    fn get_stream(&mut self) -> Receiver<GenTick<T>>;
}

/// Represents a transformation applied to ticks.
pub trait GenTickMap<T, O> {
    /// Given some settings and a `CommandServer` for logging purposes, returns a new instance of a map
    fn new(settings: HashMap<String, String>, cs: CommandServer) -> Self where Self:Sized;

    /// Processes a tick through the map, optionally returning a new tick.
    fn map(&mut self, t: GenTick<T>) -> Option<GenTick<O>>;
}

/// Represents a destination for ticks and serves as the endpoint of a tick processing pipeline.
pub trait GenTickSink<T> {
    /// Given some settings in the form of a `String`:`String` `HashMap`, returns a new instance of a sink
    fn new(settings: HashMap<String, String>) -> Result<Self, String> where Self:Sized;

    /// Processes a tick into the sink.
    fn tick(&mut self, t: GenTick<T>);
}

// some example implementations of generic tick sinks

struct PostgresSink<T> {
    buffer: Vec<GenTick<T>>,
}

// An example of specifically implementing this trait for a specific kind of data.  Since postgres requires that the
// schema of data be known in order for it to be stored, the kind of generic tick in the implementation must also be known.
impl GenTickSink<(usize, usize)> for PostgresSink<(usize, usize)> {
    fn new(settings: HashMap<String, String>) -> Result<Self, String> {
        unimplemented!();
    }

    fn tick(&mut self, t: GenTick<(usize, usize)>) {
        unimplemented!();
    }
}

struct RedisChannelSink<T> {
    buffer: Vec<GenTick<T>>,
}

// An example of generically implementing these traits for a struct.  Since Redis doesn't care the schema of the data
// you send over it (as long as you can make it into a string) any kind of generic tick is acceptable.
impl<T> GenTickSink<T> for RedisChannelSink<T> where T:Serialize, for<'de> T:Deserialize<'de> {
    fn new(settings: HashMap<String, String>) -> Result<Self, String> {
        unimplemented!();
    }

    fn tick(&mut self, t: GenTick<T>) {
        unimplemented!();
    }
}
