//! Internal server that accepts raw commands, queues them up, and transmits
//! them to the Tick Processor asynchronously.  Commands are re-transmitted
//! if a response isn't received in a timout period.
//!
//! Responses from the Tick Processor are sent back over the commands channel
//! and are sent to worker processes that register interest in them over channels.
//! Workers register interest after sending a command so that they can be notified
//! of the successful reception of the command.

//! TODO: Ensure that commands aren't processed twice by storing UIDs or most
//! recent 200 commands or something and checking that list before executing (?)

//! TODO: Use different channel for responses than for commands

use std::collections::VecDeque;
use std::thread;
use std::sync::{Arc, Mutex};

use futures::stream::{Stream, channel, Sender, Receiver};
use futures::{Future, oneshot, Complete};

type SenderQueue = Arc<Mutex<VecDeque<Sender<(Command, Complete<()>), ()>>>>;
type CommandQueue = Arc<Mutex<VecDeque<Command>>>;

/// Blocks the current thread until a Duration+Complete is received.
/// Then it sleeps for that Duration and Completes the oneshot upon awakening.
fn init_sleeper(rx: Receiver<(Duration, Complete<()>), ()>) {
    for (dur, comp) in rx.wait() {
        thread::sleep(dur);
        comp.complete(());
    }
}

/// A list of Senders over which Results from the Tick Processor
/// will be sent if they match the ID of the request the command
/// sender thread sent.
struct AlertList {
    // Receiver yeilding new messages over the responses channel
    channel_rx: Receiver<String, ()>,
    // Vec to hold the ids of responses we're waiting for and `Complete`s
    // to send the result back to the worker thread
    list: Arc<Mutex<Vec<(UID, Complete<String, ()>)>>>
}

/// Represents a response from the Tick Processor to a Command sent
/// to it at some earlier point.
enum Response {

}

impl AlertList {
    pub fn new() -> AlertList {
        let al = AlertList {
            channel_rx: sub_channel(CONF.redis_response_channel),
            list: Vec::new()
        };
        al.listen()
    }

    /// Register interest in Results with a specified UID and send
    /// the Result over the specified Oneshot when it's received
    pub fn register(&mut self, response_uid: UID, c: Complete<String, ()>) {
        self.list.push((response_uid, c));
    }

    /// Start listening on the channel and doling out Responses where requested
    fn listen(mut self) {
        let mut list = self.list.clone();
        self.channel_rx.and_then(move |raw_res| {
            // TODO: Parse the Response into a Response object
            self.send_messages(&*list, parsed_res);
        }).forget();
        self
    }

    /// Send out the Response to any workers that registered interest ot its UID
    fn send_messages(list: &Mutex<Vec<(UID, Complete<String, ()>)>>, res: Response) {
        for (uid, c) in list {
            if Response.uid == uid {
                c.complete(res);
                break;
            }
        }
    }
}

pub struct CommandServer {
    conn_count: usize, // how many connections to open
    command_queue: CommandQueue, // internal command queue
    conn_queue: SenderQueue, // senders for idle command-sender threads

}

/// Locks the CommandQueue and returns a queued command, if there are any.
fn try_get_new_command(command_queue: CommandQueue) -> Option<Command> {
    let mut qq_inner = command_queue.lock().unwrap();
    qq_inner.pop_front()
}

/// Asynchronously sends off a command to the Tick Processor without
/// waiting to see if it was received or sent properly
fn execute_command(command: Command, client: RedisClient) {
    let _ = client.execute(command.as_str(), &[]);
}

/// Creates a command processor that awaits requests
fn init_command_processor(rx: Receiver<(Command, Complete<()>), ()>, command_queue: CommandQueue) {
    // get a connection to the postgres database
    let client = get_client().expect("Couldn't create postgres connection.");
    // channel for communicating with the sleeper thread
    let (tx, rx) = channel::<(Duration, Complete<()>), ()>();
    thread::spawn(move || init_sleeper(rx) );
    // Handler for new commands from the main thread
    for tup in rx.wait() {
        let (cmd, done_tx) = tup.unwrap();
        // send off the command into the great beyond
        execute_command(cmd, &client);
        // start the timeout timer on a separate thread
        let timeout_promise = tx.send(Ok((CONF.command_timeout_duration, c)));
        // TODO: register interest in command
        timeout_promise.select(response_promise).then(|status| {
            // TODO: re-send command if timeout triggered
            // TODO: move on if it was successfully received
            // TODO: Clean out interest list if timed out
        }).wait(); // block until a response is received or the command times out
        // keep trying to get queued commands to execute until the queue is empty
        while let Some(new_command) = try_get_new_command(command_queue.clone()) {
            execute_command(new_command, &client);
        }
        // Let the main thread know it's safe to use the sender again
        // This essentially indicates that the worker thread is idle
        done_tx.complete(());
    }
}

impl CommandServer {
    pub fn new(conn_count: usize) -> CommandServer {
        let mut conn_queue = VecDeque::with_capacity(conn_count);
        let command_queue = Arc::new(Mutex::new(VecDeque::new()));
        for _ in 0..conn_count {
            // channel for getting the Sender back from the worker thread
            let (tx, rx) = channel::<(Command, Complete<()>), ()>();

            let qq_copy = command_queue.clone();
            thread::spawn(move || init_command_processor(rx, qq_copy) );
            // store the sender which can be used to send queries
            // to the worker in the connection queue
            conn_queue.push_back(tx);
        }

        CommandServer {
            conn_count: conn_count,
            command_queue: command_queue,
            conn_queue: Arc::new(Mutex::new(conn_queue))
        }
    }

    /// queues up a command to send to the Tick Processor
    pub fn execute(&mut self, command: Command) {
        // no connections available
        let temp_lock_res = self.conn_queue.lock().unwrap().is_empty();
        // Force the guard locking conn_queue to go out of scope
        // this prevents the lock from being held through the entire if/else
        let copy_res = temp_lock_res.clone();
        if copy_res {
            // push command to the command queue
            self.command_queue.lock().unwrap().push_back(command);
        }else{
            let tx = self.conn_queue.lock().unwrap().pop_front().unwrap();
            let cq_clone = self.conn_queue.clone();
            // future for notifying main thread when command is done and worker is idle
            let (c, o) = oneshot::<()>();
            tx.send(Ok((command, c))).and_then(|new_tx| {
                // Wait until the worker thread signals that it is idle
                o.and_then(move |_| {
                    // Put the Sender for the newly idle worker into the connection queue
                    cq_clone.lock().unwrap().push_back(new_tx);
                    Ok(())
                }).forget();
                Ok(())
            }).forget();
        }
    }
}
