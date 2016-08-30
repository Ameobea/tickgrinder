//! Internal server that accepts raw commands, queues them up, and transmits
//! them to the Tick Processor asynchronously.  Commands are re-transmitted
//! if a response isn't received in a timout period.
//!
//! Responses from the Tick Processor are sent back over the commands channel
//! and are sent to worker processes that register interest in them over channels.
//! Workers register interest after sending a command so that they can be notified
//! of the successful reception of the command.
//!
//! TODO: Ensure that commands aren't processed twice by storing Uuids or most
//! recent 200 commands or something and checking that list before executing (?)
//!
//! TODO: Use different channel for responses than for commands

use std::collections::VecDeque;
use std::thread::{self, Thread};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::str::FromStr;

use futures::stream::{Stream, channel, Sender, Receiver, Wait};
use futures::{Future, oneshot, Complete, Oneshot};
use uuid::Uuid;
use redis;
use serde_json;

use algobot_util::transport::redis::{get_client, sub_channel};

use conf::CONF;

/// A command waiting to be sent plus a Complete to send the Response/Errorstring through
type CommandRequest = (Command, Complete<Result<Response, String>>);
/// Threadsafe queue containing handles to idle command-sender threads in the form of Senders
type SenderQueue = Arc<Mutex<VecDeque<Sender<CommandRequest, ()>>>>;
/// Threadsafe queue containing commands waiting to be sent
type CommandQueue = Arc<Mutex<VecDeque<CommandRequest>>>;
/// A Vec containing a UUID of a Response that's expected and a Complete to send the
/// response through once it arrives
type RegisteredList = Vec<(Uuid, Complete<Result<Response, ()>>)>;
/// A message to be sent to the Timeout thread containing how long to time out for,
/// a oneshot that resolves to Err(()) if the timeout completes and a oneshot that
/// resolves to a handle to the Timeout's thread as soon as the timeout begins.
///
/// The thread handle can be used to end the timeout early to make the timeout thread
/// useable again.
type TimeoutRequest = (Duration, Complete<Result<Response, ()>>, Complete<Thread>);

/// A list of Senders over which Results from the Tick Processor will be sent if they
/// match the ID of the request the command sender thread sent.
struct AlertList {
    // Vec to hold the ids of responses we're waiting for and `Complete`s
    // to send the result back to the worker thread
    // Wrapped in Arc<Mutex<>> so that it can be accessed from within futures
    pub list: RegisteredList
}

/// Represents a command sent to the Tick Processor
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Command {
    Ping,
    Restart,
    Shutdown,
    AddSMA{period: f64},
    RemoveSMA{period: f64},
}

/// Represents a command bound to a unique identifier that can be
/// used to link it with a Response
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct WrappedCommand {
    uuid: Uuid,
    cmd: Command
}

/// Converts a String into a WrappedCommand
/// JSON Format: {"uuid": "xxxx-xxxx", "cmd": {"CommandName":{"arg": "val"}}}
pub fn parse_wrapped_command(cmd: String) -> WrappedCommand {
    serde_json::from_str::<WrappedCommand>(cmd.as_str())
        .expect("Unable to parse WrappedCommand from String")
}

/// Represents a response from the Tick Processor to a Command sent
/// to it at some earlier point.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Response {
    Ok,
    Error{status: String}
}

/// A Response bound to a UUID
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct WrappedResponse {
    pub uuid: Uuid,
    pub res: Response
}

/// Parses a String into a WrappedResponse
pub fn parse_wrapped_response(raw_res: String) -> WrappedResponse {
    serde_json::from_str::<WrappedResponse>(raw_res.as_str())
        .expect("Unable to parse WrappedResponse from String")
}

/// Send out the Response to any workers that registered interest ot its Uuid
fn send_messages(res: WrappedResponse, al: &Mutex<AlertList>) {
    let mut al_inner = al.lock().expect("Unable to unlock al n send_messages");
    let pos_opt = al_inner.list.iter_mut().position(|ref x| x.0 == res.uuid );
    match pos_opt {
        Some(pos) => {
            let (_, complete) = al_inner.list.remove(pos);
            complete.complete(Ok(res.res));
        },
        None => ()
    }
}

/// Utility struct for keeping track of the UUIDs of Responses that workers are
/// interested in and holding Completes to let them know when they are received
impl AlertList {
    pub fn new() -> AlertList {
        AlertList {
            list: Vec::new()
        }
    }

    /// Register interest in Results with a specified Uuid and send
    /// the Result over the specified Oneshot when it's received
    pub fn register(&mut self, response_uuid: &Uuid, c: Complete<Result<Response, ()>>) {
        self.list.push((response_uuid.clone(), c));
    }

    /// Deregisters a listener if a timeout in the case of a timeout occuring
    pub fn deregister(&mut self, uuid: &Uuid) {
        let pos_opt = self.list.iter().position(|x| &x.0 == uuid );
        let _ = match pos_opt {
            Some(pos) => { self.list.remove(pos); () },
            None => println!("Error removing element from interest list; it's not in it")
        };
    }
}

pub struct CommandServer {
    conn_count: usize, // how many connections to open
    command_queue: CommandQueue, // internal command queue
    conn_queue: SenderQueue, // senders for idle command-sender threads
    // alert_list: AlertList // vec of handles to workers waiting for particular Responses
}

/// Locks the CommandQueue and returns a queued command, if there are any.
fn try_get_new_command(command_queue: CommandQueue) -> Option<CommandRequest> {
    let mut qq_inner = command_queue.lock().expect("Unable to unlock qq_inner in try_get_new_command");
    qq_inner.pop_front()
}

/// Asynchronously sends off a command to the Tick Processor without
/// waiting to see if it was received or sent properly
fn send_command(cmd: &WrappedCommand, client: &mut redis::Client) {
    let command_string = serde_json::to_string(cmd)
        .expect("Unable to parse command into JSON String");
    redis::cmd("PUBLISH")
        .arg(CONF.redis_commands_channel)
        .arg(command_string)
        .execute(client);
}

/// Returns a WrappedCommand that binds a UUID to a command so that
/// Responses can be matched to it
fn wrap_command(cmd: Command) -> WrappedCommand {
    WrappedCommand {
        uuid: Uuid::new_v4(),
        cmd: cmd
    }
}

fn send_command_outer(al: &Mutex<AlertList>, wrapped_cmd: &WrappedCommand,
        client: &mut redis::Client, sleeper_tx: Sender<TimeoutRequest, ()>,
        done_c: Complete<Result<Response, String>>, command_queue: CommandQueue)
        -> Result<Sender<TimeoutRequest, ()>, ()> {
    send_command(wrapped_cmd, client);
    let (sleepy_c, sleepy_o) = oneshot::<Thread>();
    let (awake_c, awake_o) = oneshot::<Result<Response, ()>>();
    // start the timeout timer on a separate thread
    let dur = Duration::from_millis(CONF.command_timeout_ms);
    let timeout_msg = (dur, awake_c, sleepy_c);
    // sleepy_o fulfills immediately to a handle to the sleeper thread
    let sleepy_handle = sleepy_o.wait();
    let active_tx = sleeper_tx.send(Ok(timeout_msg));
    // TODO: Recycle tx
    // oneshot for sending the Response back
    let (res_recvd_c, res_recvd_o) = oneshot::<Result<Response, ()>>();
    // register interest in new Responses coming in with our Command's Uuid
    al.lock().expect("Unlock to lock al in send_command_outer")
        .register(&wrapped_cmd.uuid, res_recvd_c);
    let al_clone = al.clone();
    let mut attempts = 0;
    let _ = res_recvd_o.select(awake_o).and_then(move |res| {
        let (status, _) = res;
        // Result received before the timeout
        match status {
            // command received
            Ok(wrapped_res) => {
                // end the timeout now so that we can re-use sleeper thread
                sleepy_handle.expect("Couldn't unwrap handle to sleeper thread").unpark();
                done_c.complete(Ok(wrapped_res));
                let new_tx_wrapped = active_tx.wait();
                // keep trying to get queued commands to execute until the queue is empty
                match new_tx_wrapped {
                    Ok(new_tx) => {
                        let wrapped = try_get_new_command(command_queue.clone());
                        match wrapped {
                            Some((new_command, new_done_c)) => {
                                return Ok(Ok(send_command_outer(al, &wrap_command(new_command),
                                    client, new_tx, new_done_c, command_queue.clone())))
                            },
                            None => return Ok(Ok(Ok(new_tx)))// TODO: FULFILL idle_c
                        }
                    },
                    Err(_) => {
                        println!("Error getting new tx from old tx");
                        Ok(Err(()))
                    }
                }
            },
            // timed out
            Err(_) => {
                al_clone.lock().expect("Couldn't lock al in Err(_)").deregister(&wrapped_cmd.uuid);
                attempts += 1;
                if attempts >= CONF.max_command_retry_attempts {
                    // Let the main thread know it's safe to use the sender again
                    // This essentially indicates that the worker thread is idle
                    let err_msg = String::from_str("Timed out too many times!").unwrap();
                    done_c.complete(Err(err_msg));
                    match active_tx.wait() {
                        Ok(new_tx) => return Ok(Ok(Ok(new_tx))),
                        Err(_) => {
                            println!("Error getting new sleeper tx");
                            return Ok(Err(()))
                        }
                    }
                } else {
                    // TODO: re-send command if timeout triggered
                    // the following line is temp
                    return Ok(Err(()))
                }
            }
        }
    }).wait(); // block until a response is received or the command times out
    // Somehow nowhere else returned
    Err(())
}

/// Manually loop over the converted Stream of commands
fn manual_iterate(mut iter: Wait<Receiver<CommandRequest, ()>>, al: &Mutex<AlertList>,
        mut client: &mut redis::Client, sleeper_tx: Sender<TimeoutRequest, ()>,
        command_queue: CommandQueue) {
    let (cmd, done_c) = iter.next().expect("Coudln't unwrap #1").expect("Couldn't unwrap #2");
    println!("{:?}", cmd);
    // create a Uuid and bind it to the command
    let wrapped_cmd = WrappedCommand{uuid: Uuid::new_v4(), cmd: cmd};
    let new_sleeper_tx = send_command_outer(al, &wrapped_cmd, &mut client, sleeper_tx, done_c, command_queue.clone())
        .expect("Couldn't unwrap new_sleeper_tx");
    manual_iterate(iter, al, client, new_sleeper_tx, command_queue);
}

/// Blocks the current thread until a Duration+Complete is received.
/// Then it sleeps for that Duration and Completes the oneshot upon awakening.
/// Returns a Complete upon starting that can be used to end the timeout early
fn init_sleeper(rx: Receiver<TimeoutRequest, ()>,) {
    for res in rx.wait() {
        let (dur, awake_c, asleep_c) = res.unwrap();
        // send a Complete with a handle to the thread
        asleep_c.complete(thread::current());
        thread::park_timeout(dur);
        awake_c.complete(Err(()));
    }
}

/// Creates a command processor that awaits requests
fn init_command_processor(cmd_rx: Receiver<CommandRequest, ()>,
        command_queue: CommandQueue, al: &Mutex<AlertList>) {
    // get a connection to the postgres database
    let mut client = get_client(CONF.redis_host);
    // channel for communicating with the sleeper thread
    let (sleeper_tx, sleeper_rx) = channel::<TimeoutRequest, ()>();
    thread::spawn(move || init_sleeper(sleeper_rx) );
    let iter = cmd_rx.wait();
    manual_iterate(iter, al, &mut client, sleeper_tx, command_queue.clone());
}

impl CommandServer {
    pub fn new(conn_count: usize) -> CommandServer {
        let mut conn_queue = VecDeque::with_capacity(conn_count);
        let command_queue = Arc::new(Mutex::new(VecDeque::new()));
        let al = Arc::new(Mutex::new(AlertList::new()));
        let al_clone = al.clone();
        // Handle newly received Responses
        let rx = sub_channel(CONF.redis_host, CONF.redis_response_channel);
        rx.for_each(move |raw_res| {
            let parsed_res = parse_wrapped_response(raw_res);
            send_messages(parsed_res, &*al_clone);
            Ok(())
        }).forget();
        for _ in 0..conn_count {
            let al_clone = al.clone();
            // channel for getting the Sender back from the worker thread
            let (tx, rx) = channel::<CommandRequest, ()>();

            let qq_copy = command_queue.clone();
            thread::spawn(move || init_command_processor(rx, qq_copy, &*al_clone) );
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

    /// Queues up a command to send to the Tick Processor and returns a future
    /// that resolves to the Response returned from the Tick Processor
    pub fn execute(&mut self, command: Command) -> Oneshot<Result<Response, String>> {
        // no connections available
        let temp_lock_res = self.conn_queue.lock().unwrap().is_empty();
        // Force the guard locking conn_queue to go out of scope
        // this prevents the lock from being held through the entire if/else
        let copy_res = temp_lock_res.clone();
        // future for handing back to the caller that resolves to Response/Error
        let (res_c, res_o) = oneshot::<Result<Response, String>>();
        if copy_res {
            // push command to the command queue
            self.command_queue.lock().unwrap().push_back((command, res_c));
            // TODO: Include res_o thing
        }else{
            let tx = self.conn_queue.lock().unwrap().pop_front().unwrap();
            let cq_clone = self.conn_queue.clone();
            // future for notifying main thread when command is done and worker is idle
            let (idle_c, idle_o) = oneshot::<Result<Response, String>>();
            // TODO: SEPARATE idle_c AND res_c
            tx.send(Ok((command, idle_c))).and_then(move |new_tx| {
                // Wait until the worker thread signals that it is idle
                idle_o.and_then(move |res| {
                    res_c.complete(res);
                    // Put the Sender for the newly idle worker into the connection queue
                    cq_clone.lock().unwrap().push_back(new_tx);
                    Ok(())
                }).forget();
                Ok(())
            }).forget();
        }
        res_o
    }
}
