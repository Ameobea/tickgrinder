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

use transport::redis::{get_client, sub_channel};
use transport::commands::*;

/// Static settings for the CommandServer
#[derive(Clone, Debug)]
pub struct CsSettings {
    pub conn_count: usize,
    pub redis_host: &'static str,
    pub responses_channel: &'static str,
    pub timeout: u64,
    pub max_retries: usize
}

/// A command waiting to be sent plus a Complete to send the Response/Error String
/// through and the channel on which to broadcast the Command.
type CommandRequest = (Command, Complete<Result<Response, String>>, String);
/// Contains a CommandRequest for a worker and a Complete that resolves when the worker
/// becomes idle.
type WorkerTask = (CommandRequest, Complete<()>);
/// Threadsafe queue containing handles to idle command-sender threads in the form of Senders
type SenderQueue = Arc<Mutex<VecDeque<Sender<WorkerTask, ()>>>>;
/// Threadsafe queue containing commands waiting to be sent
type CommandQueue = Arc<Mutex<VecDeque<CommandRequest>>>;
/// A Vec containing a UUID of a Response that's expected and a Complete to send the
/// response through once it arrives
type RegisteredList = Vec<(Uuid, Complete<Result<Response, ()>>)>;
/// A message to be sent to the Timeout thread containing how long to time out for,
/// a oneshot that resolves to a handle to the Timeout's thread as soon as the timeout begins,
/// and a oneshot that resolves to Err(()) if the timeout completes.
///
/// The thread handle can be used to end the timeout early to make the timeout thread
/// useable again.
type TimeoutRequest = (Duration, Complete<Thread>, Complete<Result<Response, ()>>);

/// A list of Senders over which Results from the Tick Processor will be sent if they
/// match the ID of the request the command sender thread sent.
struct AlertList {
    // Vec to hold the ids of responses we're waiting for and `Complete`s
    // to send the result back to the worker thread
    // Wrapped in Arc<Mutex<>> so that it can be accessed from within futures
    pub list: RegisteredList
}

/// Send out the Response to any workers that registered interest to its Uuid
fn send_messages(res: WrappedResponse, al: &Mutex<AlertList>) {
    let mut to_complete = Vec::new();
    {
        let mut al_inner = al.lock().expect("Unable to unlock al n send_messages");
        let pos_opt = al_inner.list.iter_mut().position(|ref x| x.0 == res.uuid );
        match pos_opt {
            Some(pos) => {
                let (_, complete) = al_inner.list.remove(pos);
                to_complete.push( (complete, res.res) );
            },
            None => ()
        }
    }
    for (c, res) in to_complete {
        c.complete( Ok(res) );
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

#[derive(Clone)]
pub struct CommandServer {
    al: Arc<Mutex<AlertList>>,
    settings: CsSettings,
    command_queue: CommandQueue, // internal command queue
    conn_queue: SenderQueue, // senders for idle command-sender threadss
}

/// Locks the CommandQueue and returns a queued command, if there are any.
fn try_get_new_command(command_queue: CommandQueue) -> Option<CommandRequest> {
    let mut qq_inner = command_queue.lock()
        .expect("Unable to unlock qq_inner in try_get_new_command");
    qq_inner.pop_front()
}

fn send_command_outer(
    al: &Mutex<AlertList>, command: &Command, client: &mut redis::Client,
    sleeper_tx: Sender<TimeoutRequest, ()>, res_c: Complete<Result<Response, String>>,
    command_queue: CommandQueue, mut attempts: usize, s: &CsSettings,
    commands_channel: String
) -> Result<Sender<TimeoutRequest, ()>, ()> {
    let wr_cmd = command.wrap();
    let _ = send_command(&wr_cmd, client, commands_channel.as_str());

    let (sleepy_c, sleepy_o) = oneshot::<Thread>();
    let (awake_c, awake_o) = oneshot::<Result<Response, ()>>();
    // start the timeout timer on a separate thread
    let dur = Duration::from_millis(s.timeout);
    let timeout_msg = (dur, sleepy_c, awake_c);

    return sleeper_tx.send(Ok(timeout_msg)).map(move |new_sleeper_tx| {
        // sleepy_o fulfills immediately to a handle to the sleeper thread
        let sleepy_handle = sleepy_o.wait();
        // oneshot for sending the Response back
        let (res_recvd_c, res_recvd_o) = oneshot::<Result<Response, ()>>();
        // register interest in new Responses coming in with our Command's Uuid
        {
            al.lock().expect("Unlock to lock al in send_command_outer")
                .register(&wr_cmd.uuid, res_recvd_c);
        }
        let al_clone = al.clone();
        return res_recvd_o.select(awake_o).map(move |res| {
            let (status, _) = res;
            match status {
                Ok(wrapped_res) => { // command received
                    // end the timeout now so that we can re-use sleeper thread
                    sleepy_handle.expect("Couldn't unwrap handle to sleeper thread").unpark();
                    // resolve the Response future
                    res_c.complete(Ok(wrapped_res));
                    return Ok(new_sleeper_tx)
                },
                Err(_) => { // timed out
                    {
                        al_clone.lock().expect("Couldn't lock al in Err(_)")
                            .deregister(&wr_cmd.uuid);
                    }
                    attempts += 1;
                    if attempts >= s.max_retries {
                        // Let the main thread know it's safe to use the sender again
                        // This essentially indicates that the worker thread is idle
                        let err_msg = String::from_str("Timed out too many times!").unwrap();
                        res_c.complete(Err(err_msg));
                        return Ok(new_sleeper_tx)
                    } else { // re-send the command
                        // we can do this recursively since it's only a few retries
                        return send_command_outer(al, &wr_cmd.cmd, client, new_sleeper_tx, res_c,
                            command_queue.clone(), attempts, s, commands_channel.clone())
                    }
                }
            }
        }).wait() // block until a response is received or the command times out
    }).wait().ok().unwrap().ok().unwrap()
}

/// Manually loop over the converted Stream of commands
fn dispatch_worker(
    mut iter: &mut Wait<Receiver<WorkerTask, ()>>, al: &Mutex<AlertList>,
    mut client: &mut redis::Client, sleeper_tx: Sender<TimeoutRequest, ()>,
    command_queue: CommandQueue, s: &CsSettings
) -> Option<Sender<TimeoutRequest, ()>> {
    let wrapped = iter.next();

    let ((cmd, res_c, commands_channel), idle_c) = match wrapped {
        Some(inner) => inner,
        None => {
            return None;
        }
    }.expect("Couldn't unwrap #2");

    // completes initial command and internall iterates until queue is empty
    let mut new_sleeper_tx = send_command_outer(
        al, &cmd, &mut client, sleeper_tx, res_c, command_queue.clone(), 0,
        s, commands_channel
    ).expect("Couldn't unwrap new_sleeper_tx");
    // keep trying to get queued commands to execute until the queue is empty;
    while let Some((new_cmd, new_res_c, commands_channel)) =
            try_get_new_command(command_queue.clone()) {
        new_sleeper_tx = send_command_outer(
            al, &new_cmd, client, new_sleeper_tx, new_res_c, command_queue.clone(), 0, s,
            commands_channel
        ).unwrap();
    }
    idle_c.complete(());
    Some(new_sleeper_tx)
}

/// Blocks the current thread until a Duration+Complete is received.
/// Then it sleeps for that Duration and Completes the oneshot upon awakening.
/// Returns a Complete upon starting that can be used to end the timeout early
fn init_sleeper(rx: Receiver<TimeoutRequest, ()>,) {
    for res in rx.wait() {
        let (dur, asleep_c, awake_c) = res.unwrap();
        // send a Complete with a handle to the thread
        asleep_c.complete(thread::current());
        thread::park_timeout(dur);
        awake_c.complete(Err(()));
    }
}

/// Creates a command processor that awaits requests
fn init_command_processor(
    cmd_rx: Receiver<WorkerTask, ()>, command_queue: CommandQueue,
    al: &Mutex<AlertList>, s: &CsSettings
) {
    let mut client = get_client(s.redis_host);
    // channel for communicating with the sleeper thread
    let (sleeper_tx, sleeper_rx) = channel::<TimeoutRequest, ()>();
    thread::spawn(move || init_sleeper(sleeper_rx) );
    let mut iter = cmd_rx.wait();
    let mut new_sleeper_tx = dispatch_worker(
        &mut iter, al, &mut client, sleeper_tx, command_queue.clone(), &s
    );

    loop {
        // exit if we're in the process of collapse
        if new_sleeper_tx.is_none() {
            break;
        }

        new_sleeper_tx = dispatch_worker(
            &mut iter, al, &mut client, new_sleeper_tx.unwrap(), command_queue.clone(), s
        );
    }
}

impl CommandServer {
    pub fn new(s: CsSettings) -> CommandServer {
        let mut conn_queue = VecDeque::with_capacity(s.conn_count);
        let command_queue = Arc::new(Mutex::new(VecDeque::new()));
        let al = Arc::new(Mutex::new(AlertList::new()));
        let al_clone = al.clone();
        let al_clone2 = al.clone();

        // Handle newly received Responses
        let rx = sub_channel(s.redis_host, s.responses_channel);
        rx.for_each(move |raw_res| {
            let parsed_res = parse_wrapped_response(raw_res);
            send_messages(parsed_res, &*al_clone.clone());
            Ok(())
        }).forget();

        for _ in 0..s.conn_count {
            let al_clone = al.clone();
            let settings = s.clone();
            let qq_copy = command_queue.clone();

            // channel for getting the Sender back from the worker thread
            let (tx, rx) = channel::<WorkerTask, ()>();

            thread::spawn(move || init_command_processor(rx, qq_copy, &*al_clone, &settings) );
            // store the sender which can be used to send queries
            // to the worker in the connection queue
            conn_queue.push_back(tx);
        }

        CommandServer {
            al: al_clone2,
            settings: s,
            command_queue: command_queue,
            conn_queue: Arc::new(Mutex::new(conn_queue))
        }
    }

    /// Queues up a command to send to the Tick Processor.  Returns a future
    /// that resolves to the Response returned from the Tick Processor.
    pub fn execute(
        &mut self, command: Command, commands_channel: String
    ) -> Oneshot<Result<Response, String>> {
        let temp_lock_res = self.conn_queue.lock().unwrap().is_empty();
        // Force the guard locking conn_queue to go out of scope
        // this prevents the lock from being held through the entire if/else
        let copy_res = temp_lock_res.clone();
        // future for handing back to the caller that resolves to Response/Error
        let (res_c, res_o) = oneshot::<Result<Response, String>>();
        // future for notifying main thread when command is done and worker is idle
        let (idle_c, idle_o) = oneshot::<()>();

        if copy_res {
            self.command_queue.lock().unwrap().push_back((command, res_c, commands_channel));
        }else{
            let _tx = self.conn_queue.lock().unwrap().pop_front().unwrap();
            // re-assign to unlock
            let tx = _tx;
            let cq_clone = self.conn_queue.clone();
            // type WorkerTask
            let req = ((command, res_c, commands_channel), idle_c);
            tx.send(Ok(req)).and_then(move |new_tx| {
                // Wait until the worker thread signals that it is idle
                idle_o.and_then(move |_| {
                    // Put the Sender for the newly idle worker into the connection queue
                    cq_clone.lock().unwrap().push_back(new_tx);
                    Ok(())
                }).forget();
                Ok(())
            }).forget();
        }

        res_o
    }

    pub fn broadcast(
        &mut self, command: Command, commands_channel: String
    ) -> Oneshot<Result<Vec<Response>, String>> {
        // spawn a new timeout thread just for this request
        let (sleeper_tx, sleeper_rx) = channel::<TimeoutRequest, ()>();
        thread::spawn(move || init_sleeper(sleeper_rx) );
        let dur = Duration::from_millis(self.settings.timeout);

        let (sleepy_c, _) = oneshot::<Thread>();
        // awake_o fulfills when the timeout expires
        let (awake_c, awake_o) = oneshot::<Result<Response, ()>>();
        let wr_cmd = command.wrap();
        let wr_cmd_c = wr_cmd.clone();
        let mut client = get_client(self.settings.redis_host);
        // threadsafe Vec for holding returned commands
        let responses = Arc::new(Mutex::new(Vec::new()));
        let responses_c = responses.clone();
        // Oneshot for sending received responses back with.
        let (all_responses_c, all_responses_o) = oneshot::<Result<Vec<Response>, String>>();
        let alc = self.al.clone();

        // Broadcast the command
        let _ = send_command(&wr_cmd, &mut client, commands_channel.as_str());

        // when a timeout happens, send all received responses back to caller.
        awake_o.and_then(move |_| {
            let rr_inner = responses_c.lock().expect("Unable to lock responses_clone");
            let mut alc_inner = alc.lock().expect("Unable to lock alc");
            alc_inner.deregister(&wr_cmd_c.uuid);
            all_responses_c.complete( Ok( (*rr_inner).clone() ) );
            Ok(())
        }).forget();

        let timeout_msg = (dur, sleepy_c, awake_c);
        // initiate timeout
        let _ = sleeper_tx.send(Ok(timeout_msg)).wait();

        fn collect_messages(
            wr_cmd: WrappedCommand, responses: Arc<Mutex<Vec<Response>>>,
            al: Arc<Mutex<AlertList>>
        ) {
            let rrc = responses.clone();
            let alc = al.clone();

            // oneshot triggered with matching message received
            let (res_recvd_c, res_recvd_o) = oneshot::<Result<Response, ()>>();
            // register interest in new Responses coming in with our Command's Uuid
            {
                let mut al_inner = al.lock().expect("Unable to unlock to lock al in broadcast");
                al_inner.register(&wr_cmd.uuid.clone(), res_recvd_c);
            }

            res_recvd_o.and_then(move |response| {
                {
                    let mut rr_inner = rrc.lock().expect("Unable to lock responses_clone");
                    rr_inner.push(response.unwrap());
                }
                collect_messages(wr_cmd, rrc.clone(), alc);
                Ok(())
            }).forget();
        }

        collect_messages(wr_cmd.clone(), responses.clone(), self.al.clone());

        all_responses_o
    }
}
