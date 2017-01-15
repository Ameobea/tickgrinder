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

extern crate test;

use std::collections::VecDeque;
use std::thread::{self, Thread};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::str::FromStr;

use futures::{Stream, Canceled};
use futures::sync::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use futures::Future;
use futures::sync::oneshot::{channel as oneshot, Sender, Receiver};
use uuid::Uuid;
use redis;

use transport::redis::{get_client, sub_channel};
use transport::commands::*;
use conf::CONF;

/// A command waiting to be sent plus a Sender to send the Response/Error String
/// through and the channel on which to broadcast the Command.
struct CommandRequest {
    cmd: Command,
    future: Sender<Result<Response, String>>,
    channel: String,
}
/// Contains a `CommandRequest` for a worker and a Sender that resolves when the worker
/// becomes idle.
type WorkerTask = (CommandRequest, Sender<()>);
/// Threadsafe queue containing handles to idle command-sender threads in the form of `UnboundedSender`s
type UnboundedSenderQueue = Arc<Mutex<VecDeque<UnboundedSender<WorkerTask>>>>;
/// Threadsafe queue containing commands waiting to be sent
type CommandQueue = Arc<Mutex<VecDeque<CommandRequest>>>;
/// A `Vec` containing a `Uuid` of a `Response` that's expected and a `UnboundedSender` to send the
/// response through once it arrives
type RegisteredList = Vec<(Uuid, UnboundedSender<Result<Response, ()>>)>;
/// A message to be sent to the timeout thread containing how long to time out for,
/// a oneshot that resolves to a handle to the Timeout's thread as soon as the timeout begins,
/// and a oneshot that resolves to `Err(())` if the timeout completes.
///
/// The thread handle can be used to end the timeout early to make the timeout thread
/// useable again.
struct TimeoutRequest {
    dur: Duration,
    thread_future: Sender<Thread>,
    timeout_future: Sender<Result<Response, ()>>,
}

/// A list of `UnboundedSender`s over which Results from the Tick Processor will be sent if they
/// match the ID of the request the command `UnboundedSender` thread sent.
struct AlertList {
    // Vec to hold the ids of responses we're waiting for and `Sender`s
    // to send the result back to the worker thread
    // Wrapped in Arc<Mutex<>> so that it can be accessed from within futures
    pub list: RegisteredList,
}

/// Send out the Response to a worker that is registered interest to its Uuid
fn send_messages(res: WrappedResponse, al: &Mutex<AlertList>) {
    let mut al_inner = al.lock().expect("Unable to unlock al n send_messages");
    let pos_opt: Option<&mut (_, UnboundedSender<Result<Response, ()>>)> = al_inner.list.iter_mut().find(|x| x.0 == res.uuid );
    if pos_opt.is_some() {
        pos_opt.unwrap().1.send( Ok(res.res) ).expect("Unable to send through subscribed future");
    }
}

/// Utility struct for keeping track of the UUIDs of Responses that workers are
/// interested in and holding Completes to let them know when they are received
impl AlertList {
    pub fn new() -> AlertList {
        AlertList {
            list: Vec::new(),
        }
    }

    /// Register interest in Results with a specified Uuid and send
    /// the Result over the specified Oneshot when it's received
    pub fn register(&mut self, response_uuid: &Uuid, c: UnboundedSender<Result<Response, ()>>) {
        self.list.push((*response_uuid, c));
    }

    /// Deregisters a listener if a timeout in the case of a timeout occuring
    pub fn deregister(&mut self, uuid: &Uuid) {
        let pos_opt = self.list.iter().position(|x| &x.0 == uuid );
        match pos_opt {
            Some(pos) => { self.list.remove(pos); },
            None => println!("Error deregistering element from interest list; it's not in it"),
        }
    }
}

#[derive(Clone)]
pub struct CommandServer {
    al: Arc<Mutex<AlertList>>,
    command_queue: CommandQueue, // internal command queue
    conn_queue: UnboundedSenderQueue, // UnboundedSenders for idle command-UnboundedSender threadss
    client: redis::Client,
    instance: Instance, // The instance that owns this CommandServer
}

/// Locks the `CommandQueue` and returns a queued command, if there are any.
fn try_get_new_command(command_queue: CommandQueue) -> Option<CommandRequest> {
    let mut qq_inner = command_queue.lock()
        .expect("Unable to unlock qq_inner in try_get_new_command");
    qq_inner.pop_front()
}

fn send_command_outer(
    al: &Mutex<AlertList>, command: &Command, client: &mut redis::Client,
    mut sleeper_tx: &mut UnboundedSender<TimeoutRequest>, res_c: Sender<Result<Response, String>>,
    command_queue: CommandQueue, mut attempts: usize, commands_channel: String
) {
    let wr_cmd = command.wrap();
    let _ = send_command(&wr_cmd, client, commands_channel.as_str());

    let (sleepy_c, sleepy_o) = oneshot::<Thread>();
    let (awake_c, awake_o) = oneshot::<Result<Response, ()>>();
    // start the timeout timer on a separate thread
    let dur = Duration::from_millis(CONF.cs_timeout as u64);
    let timeout_msg = TimeoutRequest {
        dur: dur,
        thread_future: sleepy_c,
        timeout_future: awake_c
    };

    sleeper_tx.send(timeout_msg).unwrap();
    // sleepy_o fulfills immediately to a handle to the sleeper thread
    let sleepy_handle = sleepy_o.wait();
    // UnboundedSender for giving to the AlertList and sending the response back
    let (res_recvd_c, res_recvd_o) = unbounded::<Result<Response, ()>>();
    // register interest in new Responses coming in with our Command's Uuid
    {
        al.lock().expect("Unlock to lock al in send_command_outer #1")
            .register(&wr_cmd.uuid, res_recvd_c);
    }
    res_recvd_o.into_future().map(|(item_opt, _)| {
        item_opt.expect("item_opt was None")
    }).map_err(|_| Canceled ).select(awake_o).and_then(move |res| {
        let (status, _) = res;
        match status {
            Ok(wrapped_res) => { // command received
                {
                    // deregister since we're only waiting on one message
                    al.lock().expect("Unlock to lock al in send_command_outer #2")
                        .deregister(&wr_cmd.uuid);
                }
                // end the timeout now so that we can re-use sleeper thread
                sleepy_handle.expect("Couldn't unwrap handle to sleeper thread").unpark();
                // resolve the Response future
                res_c.complete(Ok(wrapped_res));
                return Ok(sleeper_tx)
            },
            Err(_) => { // timed out
                {
                    al.lock().expect("Couldn't lock al in Err(_)")
                        .deregister(&wr_cmd.uuid);
                }
                attempts += 1;
                if attempts >= CONF.cs_max_retries {
                    // Let the main thread know it's safe to use the UnboundedSender again
                    // This essentially indicates that the worker thread is idle
                    let err_msg = String::from_str("Timed out too many times!").unwrap();
                    res_c.complete(Err(err_msg));
                    return Ok(sleeper_tx)
                } else { // re-send the command
                    // we can do this recursively since it's only a few retries
                    send_command_outer(al, &wr_cmd.cmd, client, sleeper_tx, res_c,
                        command_queue, attempts, commands_channel)
                }
            }
        }
        Ok(sleeper_tx)
    }).wait().ok().unwrap(); // block until a response is received or the command times out
}

/// Manually loop over the converted Stream of commands
fn dispatch_worker(
    work: WorkerTask, al: &Mutex<AlertList>, mut client: &mut redis::Client,
    mut sleeper_tx: &mut UnboundedSender<TimeoutRequest>, command_queue: CommandQueue
) -> Option<()> {
    let (cr, idle_c) = work;

    // completes initial command and internally iterates until queue is empty
    send_command_outer(al, &cr.cmd, &mut client, sleeper_tx, cr.future, command_queue.clone(), 0, cr.channel);
    // keep trying to get queued commands to execute until the queue is empty;
    while let Some(cr) = try_get_new_command(command_queue.clone()) {
        send_command_outer(al, &cr.cmd, client, &mut sleeper_tx, cr.future, command_queue.clone(), 0, cr.channel);
    }
    idle_c.complete(());

    Some(())
}

/// Blocks the current thread until a Duration+Complete is received.
/// Then it sleeps for that Duration and Completes the oneshot upon awakening.
/// Returns a Complete upon starting that can be used to end the timeout early
fn init_sleeper(rx: UnboundedReceiver<TimeoutRequest>,) {
    for res in rx.wait() {
        match res.unwrap() {
            TimeoutRequest{dur, thread_future, timeout_future} => {
                // send a Complete with a handle to the thread
                thread_future.complete(thread::current());
                thread::park_timeout(dur);
                timeout_future.complete(Err(()));
            }
        }
    }
}

/// Creates a command processor that awaits requests
fn init_command_processor(
    cmd_rx: UnboundedReceiver<WorkerTask>, command_queue: CommandQueue, al: &Mutex<AlertList>
) {
    let mut client = get_client(CONF.redis_host);
    // channel for communicating with the sleeper thread
    let (mut sleeper_tx, sleeper_rx) = unbounded::<TimeoutRequest>();
    thread::spawn(move || init_sleeper(sleeper_rx) );

    for task in cmd_rx.wait() {
        let res = dispatch_worker(
            task.unwrap(), al, &mut client, &mut sleeper_tx, command_queue.clone()
        );

        // exit if we're in the process of collapse
        if res.is_none() {
            break;
        }
    }
}

impl CommandServer {
    pub fn new(instance_uuid: Uuid, instance_type: &str) -> CommandServer {
        let mut conn_queue = VecDeque::with_capacity(CONF.conn_senders);
        let command_queue = Arc::new(Mutex::new(VecDeque::new()));
        let al = Arc::new(Mutex::new(AlertList::new()));
        let al_clone = al.clone();

        // Handle newly received Responses
        let rx = sub_channel(CONF.redis_host, CONF.redis_responses_channel);
        thread::spawn(move || {
            for raw_res_res in rx.wait() {
                let raw_res = raw_res_res.expect("Res was error in CommandServer response UnboundedReceiver thread.");
                let parsed_res = parse_wrapped_response(raw_res);
                send_messages(parsed_res, &*al_clone);
            }
        });

        for _ in 0..CONF.conn_senders {
            let al_clone = al.clone();
            let qq_copy = command_queue.clone();

            // channel for getting the UnboundedSender back from the worker thread
            let (tx, rx) = unbounded::<WorkerTask>();

            thread::spawn(move || init_command_processor(rx, qq_copy, &*al_clone) );
            // store the UnboundedSender which can be used to send queries
            // to the worker in the connection queue
            conn_queue.push_back(tx);
        }

        let client = get_client(CONF.redis_host);

        CommandServer {
            al: al,
            command_queue: command_queue,
            conn_queue: Arc::new(Mutex::new(conn_queue)),
            client: client,
            instance: Instance{ uuid: instance_uuid, instance_type: String::from(instance_type), },
        }
    }

    /// Queues up a command to send to be sent.  Returns a future that resolves to
    /// the returned response.
    pub fn execute(
        &mut self, command: Command, commands_channel: String
    ) -> Receiver<Result<Response, String>> {
        let temp_lock_res = self.conn_queue.lock().unwrap().is_empty();
        // Force the guard locking conn_queue to go out of scope
        // this prevents the lock from being held through the entire if/else
        let copy_res = temp_lock_res;
        // future for handing back to the caller that resolves to Response/Error
        let (res_c, res_o) = oneshot::<Result<Response, String>>();
        // future for notifying main thread when command is done and worker is idle
        let (idle_c, idle_o) = oneshot::<()>();
        let cr = CommandRequest {
            cmd: command,
            future: res_c,
            channel: commands_channel,
        };

        if copy_res {
            self.command_queue.lock().unwrap().push_back(cr);
        }else{
            // type WorkerTask
            let req = (cr, idle_c);
            let tx;
            {
                tx = self.conn_queue.lock().unwrap().pop_front().unwrap();
                tx.send(req).unwrap();
            }
            let cq_clone = self.conn_queue.clone();
            thread::spawn(move || {
                // Wait until the worker thread signals that it is idle
                let _ = idle_o.wait();
                // Put the UnboundedSender for the newly idle worker into the connection queue
                cq_clone.lock().unwrap().push_back(tx);
            });
        }

        res_o
    }

    pub fn broadcast(
        &mut self, command: Command, commands_channel: String
    ) -> Receiver<Vec<Response>> {
        // spawn a new timeout thread just for this request
        let (sleeper_tx, sleeper_rx) = unbounded::<TimeoutRequest>();
        let dur = Duration::from_millis(CONF.cs_timeout as u64);

        let (sleepy_c, _) = oneshot::<Thread>();
        // awake_o fulfills when the timeout expires
        let (awake_c, awake_o) = oneshot::<Result<Response, ()>>();
        let wr_cmd = command.wrap();
        // Oneshot for sending received responses back with.
        let (all_responses_c, all_responses_o) = oneshot::<Vec<Response>>();

        let alc = self.al.clone();

        let (res_recvd_c, res_recvd_o) = unbounded::<Result<Response, ()>>();
        {
            // oneshot triggered with matching message received
            let mut al_inner = alc.lock().expect("Unable to unlock to lock al in broadcast");
            al_inner.register(&wr_cmd.uuid, res_recvd_c);
        }

        let responses_container = Arc::new(Mutex::new(Vec::new()));
        let responses_container_clone = responses_container.clone();
        thread::spawn(move || {
            for response in res_recvd_o.wait() {
                match response {
                    Ok(res) => {
                        let mut responses = responses_container_clone.lock().unwrap();
                        responses.push(res.expect("Inner error in responses iterator"))
                    },
                    Err(err) => println!("Got error from response iterator: {:?}", err),
                }
            }
        });

        let wr_cmd_c = wr_cmd.clone();
        thread::spawn(move || { // timer waiter thread
            // when a timeout happens, poll all the pending interest listners and send results back
            let _ = awake_o.wait();

            // deregister interest
            {
                let mut al_inner = alc.lock().expect("Unable to unlock to lock al in broadcast");
                al_inner.deregister(&wr_cmd_c.uuid);
            }

            let responses;
            {
                responses = responses_container.lock().unwrap().clone();
            }
            all_responses_c.complete(responses);
        });

        thread::spawn(move || init_sleeper(sleeper_rx) ); // timer thread

        // actually send the Command
        let _ = send_command(&wr_cmd, &self.client, commands_channel.as_str());

        let timeout_msg = TimeoutRequest {
            dur: dur,
            thread_future: sleepy_c,
            timeout_future: awake_c
        };
        // initiate timeout
        sleeper_tx.send(timeout_msg).unwrap();

        all_responses_o
    }

    /// Sends a command asynchronously without bothering to wait for responses.
    pub fn send_forget(&self, cmd: &Command, channel: &str) {
        let _ = send_command(&cmd.wrap(), &self.client, channel);
    }

    /// Sends a message to the logger with the specified severity
    pub fn log(&mut self, message_type_opt: Option<&str>, message: &str, level: LogLevel) {
        let message_type = match message_type_opt {
            Some(t) => t,
            None => "General",
        };
        let line = LogMessage {
            level: level,
            message_type: String::from(message_type),
            message: String::from(message),
            sender: self.instance.clone(),
        };
        self.send_forget(&Command::Log{msg: line}, CONF.redis_log_channel);
    }

    /// Shortcut method for logging a debug-level message.
    pub fn debug(&mut self, message_type: Option<&str>, message: &str) {
        self.log(message_type, message, LogLevel::Debug);
    }

    /// Shortcut method for logging a notice-level message.
    pub fn notice(&mut self, message_type: Option<&str>, message: &str) {
        self.log(message_type, message, LogLevel::Notice);
    }

    /// Shortcut method for logging a warning-level message.
    pub fn warning(&mut self, message_type: Option<&str>, message: &str) {
        self.log(message_type, message, LogLevel::Warning);
    }

    /// Shortcut method for logging a error-level message.
    pub fn error(&mut self, message_type: Option<&str>, message: &str) {
        self.log(message_type, message, LogLevel::Error);
    }

    /// Shortcut method for logging a critical-level message.
    pub fn critical(&mut self, message_type: Option<&str>, message: &str) {
        self.log(message_type, message, LogLevel::Critical);
    }
}

#[bench]
fn thread_spawn(b: &mut test::Bencher) {
    b.iter(|| thread::spawn(|| {}))
}
