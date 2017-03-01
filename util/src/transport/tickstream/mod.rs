//! Trait definitions and implementations for tick generators, maps, and sinks.

use std::thread::{self, Thread};
use std::time::Duration;
use std::sync::{Arc, Mutex, mpsc};
use std::sync::atomic::{Ordering, AtomicBool};
#[allow(unused_imports)]
use test;

use futures::sync::mpsc::UnboundedReceiver;
use futures::stream::BoxStream;

use trading::tick::Tick;
use transport::redis::get_client as get_redis_client;
use conf::CONF;

// readers
pub mod flatfile_reader;
pub mod postgres_reader;
pub mod random_reader;
pub mod redis_reader;

// sinks
pub mod console_sink;
pub mod null_sink;
pub mod redis_sink;
pub mod stream_sink;

// maps
pub mod maps;

// generics
pub mod generics;

pub use self::flatfile_reader::*;
pub use self::postgres_reader::*;
pub use self::random_reader::*;
pub use self::redis_reader::*;
pub use self::console_sink::*;
pub use self::null_sink::*;
pub use self::redis_sink::*;
pub use self::stream_sink::*;
pub use self::maps::*;
pub use self::generics::*;

pub type CommandStream = mpsc::Receiver<TickstreamCommand>;

/// Contains all `TickGenerator`s currently available on the platform
#[derive(Serialize, Deserialize)]
pub enum TickGenerators {
    FlatfileReader{symbol: String, start_time: Option<u64>},
    PostgresReader{symbol: String, start_time: Option<u64>},
    RandomReader,
    RedisReader{symbol: String, redis_host: String, channel: String},
}

impl TickGenerators {
    /// Depending on variant, returns a `TickGenerator` based on the supplied params.
    pub fn get(&self) -> Box<TickGenerator> {
        match self {
            &TickGenerators::FlatfileReader{ref symbol, start_time} => Box::new(FlatfileReader{symbol: symbol.clone(), start_time: start_time}),
            &TickGenerators::PostgresReader{ref symbol, start_time} => Box::new(PostgresReader{symbol: symbol.clone(), start_time: start_time}),
            &TickGenerators::RandomReader => Box::new(RandomReader {}),
            &TickGenerators::RedisReader{ref symbol, ref redis_host, ref channel} => {
                Box::new(RedisReader{symbol: symbol.clone(), redis_host: redis_host.clone(), channel: channel.clone()})
            },
        }
    }
}

/// Contains all `TickMap`s currently available on the platform
#[derive(Serialize, Deserialize)]
pub enum TickMaps {
    FastMap{delay_ms: usize},
    LiveMap{last_tick_timestamp: u64},
    NullMap,
}

impl TickMaps {
    /// Depending on variant, returns a `TickMap` based on the supplied params.
    pub fn get(&self) -> Box<TickMap> {
        match self {
            &TickMaps::FastMap{delay_ms} => Box::new(FastMap{delay_ms: delay_ms}),
            &TickMaps::LiveMap{last_tick_timestamp} => Box::new(LiveMap{last_tick_timestamp: last_tick_timestamp}),
            &TickMaps::NullMap => Box::new(NullMap {}),
        }
    }
}

/// Contains all `TickSink`s currently available on the platform
#[derive(Serialize, Deserialize)]
pub enum TickSinks {
    ConsoleSink,
    NullSink,
    RedisSink{symbol: String, tx_channel: String},
}

impl TickSinks {
    /// Depending on variant, returns a `TickSink` based on the supplied params.
    pub fn get(&self) -> Box<TickSink> {
        match self {
            &TickSinks::ConsoleSink => Box::new(ConsoleSink {}),
            &TickSinks::NullSink => Box::new(NullSink {}),
            &TickSinks::RedisSink{ref symbol, ref tx_channel} => {
                let client = get_redis_client(CONF.redis_host);
                Box::new(RedisSink{client: client, symbol: symbol.clone(), tx_channel: tx_channel.clone()})
            },
        }
    }
}

/// Commands for controlling the flow of ticks in a tickstream
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TickstreamCommand {
    Pause,
    Resume,
    Stop,
}

/// Creates a Stream of Ticks from some source.
pub trait TickGenerator {
    /// Returns a stream that resolves to new Ticks
    fn get(
        &mut self, map: Box<TickMap + Send>, cmd_handle: CommandStream
    ) -> Result<BoxStream<Tick, ()>, String>;

    /// Returns a stream that yields the generator's ticks without any map or command handler.
    fn get_raw(&mut self) -> Result<BoxStream<Tick, ()>, String>;
}

/// Represents an endpoint through which ticks generated in a Backtest can be sent.
///
/// Could be, for example, a Redis channel, IPC bus, database, etc.
pub trait TickSink {
    /// Called every time a new tick is available from the Backtest
    fn tick(&mut self, t: Tick);
}

/// Function called in between the `TickGenerator` and the `TickSink`.  Used to do things like add
/// latency, simulate slippage/lost ticks, etc.
pub trait TickMap {
    fn map(&mut self, Tick) -> Option<Tick>;
}

/// Handles backtest messages within the backtest's worker thread.  If this returns true,
/// it means the caller has to die.
pub fn check_mail(got_mail: &AtomicBool, message: &Mutex<TickstreamCommand>) -> bool {
    // nothing to do if we have no new commands to process
    if !got_mail.load(Ordering::Relaxed) { return false }

    let cur_command: TickstreamCommand;
    {
        let atom = message.lock().unwrap();
        // handing the command may block for a long time, so make sure mutex is unlocked first.
        cur_command = atom.clone();
    }

    match cur_command {
        TickstreamCommand::Pause => thread::park(),
        TickstreamCommand::Stop => return true,
        // _ => println!("Backtest worker can't handle command: {:?}", cur_command),
        _ => (),
    }

    got_mail.store(false, Ordering::Relaxed);

    false
}

/// Spawns the thread that listens for new `TickstreamCommand`s and relays them
/// internally to the worker thread.
pub fn spawn_listener_thread(
    got_mail: Arc<AtomicBool>, cmd_handle: CommandStream,
    internal_message: Arc<Mutex<TickstreamCommand>>, reader_handle: Thread
) {
    thread::spawn(move || {
        // block until new backtest command received
        for cmd in cmd_handle.iter() {
            // println!("Received backtest command: {:?}", cmd);
            match cmd {
                TickstreamCommand::Stop | TickstreamCommand::Pause => {
                    let mut lock = internal_message.lock().unwrap();
                    *lock = cmd.clone();
                    got_mail.store(true, Ordering::Relaxed);
                },
                TickstreamCommand::Resume => reader_handle.unpark(),
            }
        }
    });
}

/// See how fast we can check the value of the atomic bool
#[bench]
fn mail_check_no_messages(b: &mut test::Bencher) {
    let msg = Mutex::new(TickstreamCommand::Stop);
    b.iter(|| {
        check_mail(&AtomicBool::new(false), &msg)
    })
}

/// How fast we can check the atomic bool and unlock the mutex
#[bench]
fn mail_check_messages(b: &mut test::Bencher) {
    let msg = Mutex::new(TickstreamCommand::Stop);
    b.iter(|| {
        check_mail(&AtomicBool::new(true), &msg)
    })
}
