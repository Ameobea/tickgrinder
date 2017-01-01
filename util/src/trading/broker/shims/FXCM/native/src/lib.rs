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
    fn init_broker_server(session: *mut c_void, cb: Option<extern fn (tx_ptr: *mut c_void, message: *mut ServerMessage)>) -> *mut c_void;
    fn exec_command(command: ServerCommand, args: *mut c_void, server_env: *mut c_void);
}

/// Contains all possible commands that can be received by the broker server.
#[repr(C)]
enum ServerCommand {
    MARKET_OPEN,
    MARKET_CLOSE,
    LIST_ACCOUNTS,
    DISCONNECT,
}

/// Contains all possible responses that can be received by the broker server.
#[repr(C)]
enum ServerResponse {
    POSITION_OPENED,
    POSITION_CLOSED,
    ORDER_PLACED,
    ORDER_REMOVED,
    SESSION_TERMINATED,
}

/// A packet of information asynchronously received from the broker server.
#[repr(C)]
struct ServerMessage {
    response: ServerResponse,
    payload: *mut c_void,
}

pub struct FXCMNative {
    settings_hash: HashMap<String, String>,
    server_environment: *mut c_void,
}

unsafe impl Send for FXCMNative {}

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

        sender.send(res);
    }
}

impl Broker for FXCMNative {
    fn init(settings: HashMap<String, String>) -> Receiver<Result<Self, BrokerError>> where Self:Sized {
        let (tx, rx) = oneshot::channel::<Result<Self, BrokerError>>();
        thread::spawn(move || {
            let session: *mut c_void = login();
            let server_environment: *mut c_void = unsafe { init_broker_server(session, Some(handle_message)) };

            let inst = FXCMNative {
                settings_hash: settings,
                server_environment: server_environment,
            };

            tx.complete(Ok(inst));
        });

        rx
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
