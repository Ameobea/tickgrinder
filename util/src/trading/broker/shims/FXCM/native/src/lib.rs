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
use std::mem::transmute;

use libc::{c_char, c_void, uint64_t, c_double};
use uuid::Uuid;
use futures::Oneshot;
use futures::stream::Stream;
use futures::sync::oneshot::Receiver;
use futures::sync::mpsc::{UnboundedSender, UnboundedReceiver};

use algobot_util::trading::broker::*;
use algobot_util::trading::tick::*;

mod conf;
use conf::CONF;

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
}

pub struct FXCMNative {
    settings_hash: HashMap<String, String>,
}

impl FXCMNative {
    pub fn new(settings: HashMap<String, String>) -> FXCMNative {
        FXCMNative {
            settings_hash: settings,
        }
    }
}

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

impl Broker for FXCMNative {
    fn init(&mut self, settings: HashMap<String, String>) -> Receiver<Result<Self, BrokerError>> where Self:Sized {
        unimplemented!();
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
