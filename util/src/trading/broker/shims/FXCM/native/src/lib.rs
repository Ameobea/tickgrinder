//! Broker shim for FXCM.

#![feature(libc)]

extern crate uuid;
extern crate futures;
extern crate algobot_util;
extern crate libc;

use std::collections::HashMap;
use std::ffi::CString;

use libc::c_char;
use uuid::Uuid;
use futures::Oneshot;
use futures::stream::{Stream, Receiver};

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
    fn fxcm_login(username: *const c_char, password: *const c_char, url: *const c_char, connection_name: *const c_char, session_id: *const c_char, pin: *const c_char);
}

// TODO: Import functions from the external C++ library created in native

#[repr(C)]
pub struct FXCMNative {
    settings_hash: HashMap<String, String>,
}

impl FXCMNative {
    pub fn new(settings: HashMap<String, String>) -> FXCMNative {
        unsafe {
            let username = CString::new(CONF.fxcm_username).unwrap().as_ptr();
            fxcm_login(username, username, username, username, username, username);
        }
        FXCMNative {
            settings_hash: settings,
        }
    }
}

impl Broker for FXCMNative {
    fn init(&mut self, settings: HashMap<String, String>) -> Oneshot<Result<Self, BrokerError>> where Self:Sized {
        unimplemented!();
    }

    fn list_accounts(&mut self) -> Oneshot<Result<HashMap<Uuid, Account>, BrokerError>> {
        unimplemented!();
    }

    fn get_ledger(&mut self, account_id: Uuid) -> Oneshot<Result<Ledger, BrokerError>> {
        unimplemented!();
    }

    fn execute(&mut self, action: BrokerAction) -> PendingResult {
        unimplemented!();
    }

    fn get_stream(&mut self) -> Result<Receiver<BrokerMessage, BrokerError>, BrokerError> {
        unimplemented!();
    }

    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()> + Send>, BrokerError> {
        unimplemented!();
    }
}

/// Tests the ability to log in to FXCM via the C++ code in the library.
#[test]
fn login_test() {
    unsafe {
        // TODO: Either convert these to the correct format or do so on the C++ end
        // currently things are looking weird:
        // HTTP request failed object='/?ID=1479458729492&PN=���^&SN=ForexConnect&MV=5&LN=���^&AT=PLAIN' errorCode=0
        let username  = CString::new(CONF.fxcm_username).unwrap().as_ptr();
        let password  = CString::new(CONF.fxcm_password).unwrap().as_ptr();
        let url       = CString::new(CONF.fxcm_url).unwrap().as_ptr();
        let conn_name = CString::new("Test Connection").unwrap().as_ptr();
        let null_str  = CString::new("").unwrap().as_ptr();
        fxcm_login(username, password, url, conn_name, null_str, null_str);
    }
}
