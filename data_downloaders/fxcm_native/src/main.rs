//! FXCM Native Data Downloader
//!
//! See README.txt for more information

#![feature(libc)]
#![allow(dead_code)]

extern crate libc;
extern crate algobot_util;

use std::thread;
use std::sync::mpsc::{channel, Sender};
use std::ffi::CString;

use libc::{c_char, c_void, uint64_t};

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
        tick_callback: extern fn (*mut c_void, uint64_t, uint64_t, uint64_t),
        user_data: *mut c_void
    );
}

#[repr(C)]
struct DataDownloader {
    tx: Sender<Tick>,
}

impl DataDownloader {
    pub fn new() -> DataDownloader {
        let (tx, rx) = channel::<Tick>();

        thread::spawn(move || {
            for msg in rx.iter() {
                println!("{:?}", msg);
            }
        });

        DataDownloader {
            tx: tx,
        }
    }

    pub fn init_download<F>(
        &mut self, symbol: &str, tick_callback: F
    ) where F: FnMut(uint64_t, uint64_t, uint64_t) {
        let username  = CString::new(CONF.fxcm_username).unwrap();
        let password  = CString::new(CONF.fxcm_password).unwrap();
        let url       = CString::new(CONF.fxcm_url).unwrap();
        let symbol    = CString::new(symbol).unwrap();
        unsafe {
            let session_ptr = fxcm_login(username.as_ptr(), password.as_ptr(), url.as_ptr(), false);
            let user_data = &tick_callback as *const _ as *mut c_void;
            init_history_download(session_ptr, symbol.as_ptr(), tick_callback_shim::<F>, user_data);
        }
    }

    pub extern "C" fn test(&mut self, timestamp: uint64_t, bid: uint64_t, ask: uint64_t) {
        let t = Tick {
            timestamp: timestamp as usize,
            bid: bid as usize,
            ask: ask as usize,
        };
        let _ = self.tx.send(t);
    }
}

pub extern "C" fn tick_callback_shim<F>(
    closure: *mut c_void, timestamp: uint64_t, bid: uint64_t, ask: uint64_t
) where F: FnMut(uint64_t, uint64_t, uint64_t) {
    let opt_closure = closure as *mut Option<F>;
    unsafe {
        (*opt_closure).take().unwrap()(timestamp, bid, ask);
    }
}

fn main() {
    let mut downloader = DataDownloader::new();
    let tx = downloader.tx.clone();
    let cb_closure = |timestamp: uint64_t, bid: uint64_t, ask: uint64_t| {
        let t = Tick {
            timestamp: timestamp as usize,
            bid: bid as usize,
            ask: ask as usize,
        };
        let _ = tx.send(t);
    };
    downloader.init_download("EURUSD", cb_closure);
}
