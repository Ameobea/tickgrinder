//! Broker shim for FXCM.

#![feature(libc)]
#![allow(dead_code, unused_imports, unused_variables)]

extern crate uuid;
extern crate futures;
extern crate algobot_util;
extern crate libc;

use std::collections::HashMap;
use std::ffi::CString;
use std::ptr::null;
use std::thread;
use std::mem::transmute;
use std::sync::mpsc;
use std::time::Duration;

use libc::{c_char, c_void, uint64_t, c_double};
use uuid::Uuid;
use futures::Oneshot;
use futures::sync::oneshot;
use futures::stream::Stream;
use futures::sync::oneshot::Receiver;
use futures::sync::mpsc::{UnboundedSender, UnboundedReceiver};

use algobot_util::trading::broker::*;
use algobot_util::trading::tick::*;
use algobot_util::conf::CONF;

#[link(name="fxtp")]
#[link(name="gsexpat")]
#[link(name="gstool3")]
#[link(name="httplib")]
#[link(name="log4cplus")]
#[link(name="pdas")]
#[link(name="fxcm_ffi")]
#[link(name="stdc++")]
#[link(name="fxmsg")]
#[link(name="ForexConnect")]
#[link(name="sample_tools")]
extern {
    fn fxcm_login(username: *const c_char, password: *const c_char, url: *const c_char, live: bool) -> *mut c_void;
    fn test_login(username: *const c_char, password: *const c_char, url: *const c_char, live: bool) -> bool;
    fn init_history_download(
        connection: *mut c_void,
        symbol: *const c_char,
        start_time: *const c_char,
        end_time: *const c_char,
        tick_callback: extern fn (*mut c_void, uint64_t, c_double, c_double),
        user_data: *mut c_void
    );

    // broker server commands
    fn init_server_environment(cb: Option<extern fn (tx_ptr: *mut c_void, message: *mut ServerMessage)>, tx_ptr: *mut c_void) -> *mut c_void;
    fn start_server(session: *mut c_void, env: *mut c_void);
    fn exec_command(command: ServerCommand, args: *mut c_void, server_env: *mut c_void);
    fn push_client_message(message: ClientMessage, env: *mut c_void);
}

/// Contains all possible commands that can be received by the broker server.
#[repr(C)]
#[derive(Clone)]
enum ServerCommand {
    MARKET_OPEN,
    MARKET_CLOSE,
    LIST_ACCOUNTS,
    DISCONNECT,
    PING,
}

/// Contains all possible responses that can be received by the broker server.
#[repr(C)]
#[derive(Clone)]
enum ServerResponse {
    POSITION_OPENED,
    POSITION_CLOSED,
    ORDER_PLACED,
    ORDER_REMOVED,
    SESSION_TERMINATED,
}

/// A packet of information asynchronously received from the broker server.
#[repr(C)]
#[derive(Clone)]
pub struct ServerMessage {
    response: ServerResponse,
    payload: *mut c_void,
}

/// A packet of information that can be sent to the broker server.
#[repr(C)]
#[derive(Clone)]
pub struct ClientMessage {
    command: ServerCommand,
    payload: *mut c_void,
}

pub struct FXCMNative {
    settings_hash: HashMap<String, String>,
    server_environment: *mut c_void,
}

// something to hold our environment so we can convince Rust to send it between threads
#[derive(Clone)]
struct Spaceship(*mut c_void);

unsafe impl Send for Spaceship{}
unsafe impl Send for FXCMNative {}
unsafe impl Send for ServerMessage {}
unsafe impl Send for ClientMessage {}

/// Called for every historical tick downloaded by the `init_history_download` function.  This function is called
/// asynchronously from within the C++ code of the native FXCM broker library.
#[no_mangle]
pub extern fn tick_downloader_cb(timestamp: uint64_t, bid: uint64_t, ask: uint64_t){
    let t = Tick {
        timestamp: timestamp as usize,
        bid: bid as usize,
        ask: ask as usize
    };
    println!("{:?}", t);
}

/// Processes received messages from the broker server and converts them into BrokerResults that can be fed to the
/// stream returned by `get_stream`.
extern fn handle_message(tx_ptr: *mut c_void, message: *mut ServerMessage) {
    unsafe {
        let mut sender: &mut UnboundedSender<BrokerResult> = transmute(tx_ptr);
        let res: BrokerResult = match (*message).response {
            ServerResponse::POSITION_OPENED => {
                unimplemented!();
            },
            _ => unimplemented!(),
        };

        let _ = sender.send(res);
    }
}

impl Broker for FXCMNative {
    fn init(settings: HashMap<String, String>) -> Receiver<Result<Self, BrokerError>> where Self:Sized {
        let (ext_tx, ext_rx) = oneshot::channel::<Result<Self, BrokerError>>();
        thread::spawn(move || {
            // channel with which to receive messages from the server
            let (tx, rx) = mpsc::channel::<ServerMessage>();
            let tx_ptr = &tx as *const _ as *mut c_void;

            let server_environment: *mut c_void = unsafe { init_server_environment(Some(handle_message), tx_ptr) };
            let ship = Spaceship(server_environment.clone());

            thread::spawn(move || {
                let session: *mut c_void = login();
                // blocks on C++ event loop
                unsafe { start_server(session, ship.0) };
            });

            let inst = FXCMNative {
                settings_hash: settings,
                server_environment: server_environment,
            };

            ext_tx.complete(Ok(inst));
        });

        ext_rx
    }

    fn list_accounts(&mut self) -> Receiver<Result<HashMap<Uuid, Account>, BrokerError>> {
        unimplemented!();
    }

    fn get_ledger(&mut self, account_id: Uuid) -> Receiver<Result<Ledger, BrokerError>> {
        unimplemented!();
    }

    fn execute(&mut self, action: BrokerAction) -> PendingResult {
        unimplemented!();
    }

    fn get_stream(&mut self) -> Result<UnboundedReceiver<BrokerResult>, BrokerError> {
        unimplemented!();
    }

    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()> + Send>, BrokerError> {
        unimplemented!();
    }
}

fn login() -> *mut c_void {
    let username  = CString::new(CONF.fxcm_username).unwrap();
    let password  = CString::new(CONF.fxcm_password).unwrap();
    let url       = CString::new(CONF.fxcm_url).unwrap();

    unsafe { fxcm_login(username.as_ptr(), password.as_ptr(), url.as_ptr(), false) }
}

/// Tests the ability to log in to FXCM via the C++ code in the library.
#[test]
fn login_test() {
    let username      = CString::new(CONF.fxcm_username).unwrap();
    let mut password  = CString::new(CONF.fxcm_password).unwrap();
    let url           = CString::new(CONF.fxcm_url).unwrap();
    let mut success: bool;
    unsafe {
        success = test_login(username.as_ptr(), password.as_ptr(), url.as_ptr(), false);
    }
    assert!(success, "Error during remote function call; unable to connect to broker.");

    password = CString::new("wrongpassword").unwrap();
    unsafe {
        success = test_login(username.as_ptr(), password.as_ptr(), url.as_ptr(), false);
    }
    assert!(!success, "Test function returned true even for bad credentials.");
}

#[test]
fn broker_server() {
    // channel with which to receive responses from the server
    let (tx, rx) = mpsc::channel::<ServerMessage>();
    let tx_ptr = &tx as *const _ as *mut c_void;

    let env: *mut c_void = unsafe { init_server_environment(Some(handle_message), tx_ptr) };
    let ship  = Spaceship(env);
    let ship2 = ship.clone();

    let handle = thread::spawn(move || {
        // TODO: wait until the connection is ready before starting to process messages
        // let session = login();
        // block on the C++ event loop code and start processing messages
        unsafe { start_server(0 as *mut c_void, ship.0) };
        println!("WTF");
    });

    let message = ClientMessage {
        command: ServerCommand::PING,
        payload: 0 as *mut c_void,
    };

    thread::spawn(move || {
        for i in 0..10 {
            println!("Sending message on rust side...");
            unsafe { push_client_message(message.clone(), ship2.0) };
        }
    });

    thread::park();
}
