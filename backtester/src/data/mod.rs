//! Supplies the backtester with historical ticks stored in a variety of formats.

use std::sync::atomic::{Ordering, AtomicBool};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::thread::Thread;
#[allow(unused_imports)]
use test;

use futures::stream::Receiver;
use algobot_util::trading::tick::Tick;

use backtest::BacktestMap;

pub mod flatfile_reader;
pub mod redis_reader;
pub mod random_reader;
pub mod redis_sink;
pub mod console_sink;
pub mod null_sink;
pub mod stream_sink;

pub use self::flatfile_reader::*;
pub use self::redis_reader::*;
pub use self::random_reader::*;
pub use self::redis_sink::RedisSink;
pub use self::console_sink::ConsoleSink;
pub use self::null_sink::NullSink;
pub use self::stream_sink::StreamSink;
use backtest::BacktestCommand;

pub type CommandStream = mpsc::Receiver<BacktestCommand>;

/// Creates a Stream of Ticks to feed the backtest.
pub trait TickGenerator {
    /// Returns a stream that resolves to new Ticks
    fn get(&mut self, map: Box<BacktestMap + Send>, cmd_handle: CommandStream)
        -> Result<Receiver<Tick, ()>, String>;
}

/// Handles backtest messages within the backtest's worker thread.  If this returns true,
/// it means the caller has to die.
pub fn check_mail(got_mail: &AtomicBool, message: &Mutex<BacktestCommand>) -> bool {
    // nothing to do if we have no new commands to process
    if !got_mail.load(Ordering::Relaxed) { return false }

    let cur_command: BacktestCommand;
    {
        let atom = message.lock().unwrap();
        // handing the command may block for a long time, so make sure mutex is unlocked first.
        cur_command = atom.clone();
    }

    match cur_command {
        BacktestCommand::Pause => thread::park(),
        BacktestCommand::Stop => return true,
        _ => println!("Backtest worker can't handle command: {:?}", cur_command),
    }

    got_mail.store(false, Ordering::Relaxed);

    false
}

/// Spawns the thread that listens for new BacktestCommands and relays them
/// internally to the worker thread.
pub fn spawn_listener_thread(
    got_mail: Arc<AtomicBool>, cmd_handle: CommandStream,
    internal_message: Arc<Mutex<BacktestCommand>>, reader_handle: Thread
) {
    thread::spawn(move || {
        // block until new backtest command received
        for cmd in cmd_handle.iter() {
            println!("Received backtest command: {:?}", cmd);
            match cmd {
                BacktestCommand::Stop | BacktestCommand::Pause => {
                    let mut lock = internal_message.lock().unwrap();
                    *lock = cmd.clone();
                    got_mail.store(true, Ordering::Relaxed);
                },
                BacktestCommand::Resume => reader_handle.unpark(),
                // _ => println!("Commmand not implemented by Flatfile Reader: {:?}", cmd),
            }
        }
    });
}

/// Represents an endpoint through which ticks generated in a Backtest can be sent.
///
/// Could be, for example, a Redis channel, IPC bus, database, etc.
pub trait TickSink {
    /// Called every time a new tick is available from the Backtest
    fn tick(&mut self, t: Tick);
}

/// See how fast we can check the value of the atomic bool
#[bench]
fn mail_check_no_messages(b: &mut test::Bencher) {
    let msg = Mutex::new(BacktestCommand::Stop);
    b.iter(|| {
        check_mail(&AtomicBool::new(false), &msg)
    })
}

/// How fast we can check the atomic bool and unlock the mutex
#[bench]
fn mail_check_messages(b: &mut test::Bencher) {
    let msg = Mutex::new(BacktestCommand::Stop);
    b.iter(|| {
        check_mail(&AtomicBool::new(true), &msg)
    })
}
