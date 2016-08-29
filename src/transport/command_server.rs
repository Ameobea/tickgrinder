//! Internal server that accepts raw commands, queues them up, and transmits
//! them to the Tick Processor asynchronously.  Commands are re-transmitted
//! if a response isn't received in a timout period.
//!
//! Responses from the Tick Processor are sent back over the commands channel
//! and are sent to worker processes that register interest in them over channels.
//! Workers register interest after sending a command so that they can be notified
//! of the successful reception of the command.

//! TODO: Ensure that commands aren't processed twice by storing Uuids or most
//! recent 200 commands or something and checking that list before executing (?)

//! TODO: Use different channel for responses than for commands

use std::collections::VecDeque;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

use futures::stream::{Stream, channel, Sender, Receiver};
use futures::{Future, oneshot, Complete};
use uuid::Uuid;
use redis;

use algobot_util::transport::redis::{get_client, sub_channel};

use conf::CONF;

type SenderQueue = Arc<Mutex<VecDeque<Sender<(Command, Complete<()>), ()>>>>;
type CommandQueue = Arc<Mutex<VecDeque<Command>>>;

/// Blocks the current thread until a Duration+Complete is received.
/// Then it sleeps for that Duration and Completes the oneshot upon awakening.
fn init_sleeper(rx: Receiver<(Duration, Complete<()>), ()>) {
    for res in rx.wait() {
        let (dur, comp) = res.unwrap();
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
    list: Vec<(Uuid, Complete<Result<WrappedResponse, ()>>)>
}

/// Represents a command sent to the Tick Processor
enum Command {

}

/// Represents a command bound to a unique identifier that can be
/// used to link it with a Response
struct WrappedCommand {
    uuid: Uuid,
    cmd: Command
}

/// Represents a response from the Tick Processor to a Command sent
/// to it at some earlier point.
enum Response {
    Ok,
    Error{status: String}
}

struct WrappedResponse {
    pub uuid: Uuid,
    pub cmd: Response
}

impl AlertList {
    pub fn new() -> AlertList {
        let al = AlertList {
            channel_rx: sub_channel(CONF.redis_host, CONF.redis_response_channel),
            list: Vec::new()
        };
        al.listen()
    }

    /// Register interest in Results with a specified Uuid and send
    /// the Result over the specified Oneshot when it's received
    pub fn register(&mut self, response_uuid: Uuid, c: Complete<Result<WrappedResponse, ()>>) {
        self.list.push((response_uuid, c));
    }

    /// Deregisters a listener if a timeout in the case of a timeout occuring
    pub fn deregister(&mut self, response_uuid: Uuid) {

    }

    /// Start listening on the channel and doling out Responses where requested
    fn listen(mut self) -> AlertList {
        self.channel_rx.and_then(move |raw_res| {
            // TODO: Parse the Response into a Response object
            self.send_messages(parsed_res);
            Ok(())
        }).forget();
        self
    }

    /// Send out the Response to any workers that registered interest ot its Uuid
    fn send_messages(&mut self, list:  res: WrappedResponse) {
        for (i, &(Uuid, c)) in self.list.iter().enumerate() {
            if res.uuid == Uuid {
                self.list.remove(i);
                c.complete(Ok(res));
                break;
            }
        }
    }
}

pub struct CommandServer {
    conn_count: usize, // how many connections to open
    command_queue: CommandQueue, // internal command queue
    conn_queue: SenderQueue, // senders for idle command-sender threads
    alert_list: Arc<Mutex<AlertList>> // vec of handles to workers waiting for particular Responses
}

/// Locks the CommandQueue and returns a queued command, if there are any.
fn try_get_new_command(command_queue: CommandQueue) -> Option<Command> {
    let mut qq_inner = command_queue.lock().unwrap();
    qq_inner.pop_front()
}

/// Asynchronously sends off a command to the Tick Processor without
/// waiting to see if it was received or sent properly
fn execute_command(cmd: WrappedCommand, client: &redis::Client) {
    // TODO: Send command over redis
}

/// Returns a WrappedCommand that binds a UUID to a command so that
/// Responses can be matched to it
fn wrap_command(cmd: Command) -> WrappedCommand {
    WrappedCommand {
        uuid: Uuid::new_v4(),
        cmd: cmd
    }
}

/// Creates a command processor that awaits requests
fn init_command_processor(cmd_rx: Receiver<(Command, Complete<()>), ()>,
        command_queue: CommandQueue, al: &Mutex<AlertList>) {
    // get a connection to the postgres database
    let client = get_client(CONF.redis_host);
    // channel for communicating with the sleeper thread
    let (tx, rx) = channel::<(Duration, Complete<()>), ()>();
    thread::spawn(move || init_sleeper(rx) );
    for tup in cmd_rx.wait() {
        let (cmd, done_tx) = tup.unwrap();
        // create a Uuid and bind it to the command
        let wrapped_cmd = WrappedCommand{uuid: Uuid::new_v4(), cmd: cmd};
        execute_command(wrapped_cmd, &client);
        // `timeout` fulfills when the timeout is up
        let (c, timeout) = oneshot::<()>();
        // start the timeout timer on a separate thread
        let timeout_promise = tx.send(Ok((CONF.command_timeout_duration, c)));
        // TODO: Recycle tx
        // oneshot for sending the Response back
        let (complete, oneshot) = oneshot::<Result<WrappedResponse, ()>>();
        let mut al_inner = al.lock().unwrap();
        // register interest in new Responses coming in with our Command's Uuid
        al_inner.register(wrapped_cmd.uuid, complete);
        let mut attempts = 0;
        timeout.select(oneshot).and_then(move |status| {
            // Result received before the timeout
            match status.unwrap() {
                // command received
                Ok(raw_res) => {
                    // TODO: Parse into WrappedResponse + Result and return
                    al_inner
                },
                // timed out
                Err(_) => {
                    // TODO
                }
            }
            // TODO: re-send command if timeout triggered
            // TODO: move on if it was successfully received
            // TODO: Clean out interest list if timed out
        }).wait(); // block until a response is received or the command times out
        // keep trying to get queued commands to execute until the queue is empty
        while let Some(new_command) = try_get_new_command(command_queue.clone()) {
            execute_command(wrap_command(new_command), &client);
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
        let al = Arc::new(Mutex::new(AlertList::new()));
        for _ in 0..conn_count {
            let al = al.clone();
            // channel for getting the Sender back from the worker thread
            let (tx, rx) = channel::<(Command, Complete<()>), ()>();

            let qq_copy = command_queue.clone();
            thread::spawn(move || init_command_processor(rx, qq_copy, &*al) );
            // store the sender which can be used to send queries
            // to the worker in the connection queue
            conn_queue.push_back(tx);
        }

        CommandServer {
            conn_count: conn_count,
            command_queue: command_queue,
            conn_queue: Arc::new(Mutex::new(conn_queue)),
            alert_list: al
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
            let (complete, oneshot) = oneshot::<()>();
            tx.send(Ok((command, complete))).and_then(|new_tx| {
                // Wait until the worker thread signals that it is idle
                oneshot.and_then(move |_| {
                    // Put the Sender for the newly idle worker into the connection queue
                    cq_clone.lock().unwrap().push_back(new_tx);
                    Ok(())
                }).forget();
                Ok(())
            }).forget();
        }
    }
}
