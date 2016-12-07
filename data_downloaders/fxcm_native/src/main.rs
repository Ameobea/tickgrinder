//! FXCM Native Data Downloader
//!
//! See README.txt for more information

#![feature(libc, conservative_impl_trait, fn_traits, unboxed_closures)]

extern crate libc;
extern crate algobot_util;
extern crate redis;
extern crate serde_json;
extern crate time;

use std::thread;
use std::sync::mpsc::{channel, Sender};
use std::ffi::CString;
use std::mem::transmute;

use libc::{c_char, c_void, uint64_t, c_double};

use algobot_util::transport::redis::*;
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
    fn init_history_download(
        connection: *mut c_void,
        symbol: *const c_char,
        start_time: *const c_char,
        end_time: *const c_char,
        tick_callback: Option<extern "C" fn (*mut c_void, uint64_t, c_double, c_double)>,
        user_data: *mut c_void
    );
}

#[derive(Debug)]
#[repr(C)]
struct CTick {
    timestamp: uint64_t,
    bid: c_double,
    ask: c_double,
}

impl CTick {
    pub fn to_tick(&self, decimals: usize) -> Tick {
        Tick {
            timestamp: self.timestamp as usize,
            bid: (self.bid as f64 as f64 * decimals as f64) as usize,
            ask: (self.ask as f64 as f64 * decimals as f64) as usize,
        }
    }
}

struct DataDownloader {}

/// Where to save the recorded ticks to.
pub enum DataDst {
    Flatfile{filename: String},
    Postgres{database: String, table: String},
    Redis{host: String, channel: String},
    Console,
}

impl DataDownloader {
    pub fn new() -> DataDownloader {
        DataDownloader {}
    }

    pub fn init_download<F>(
        &mut self, symbol: &str, dst: DataDst, start_time: &str, end_time: &str,
    ) where F: FnMut(uint64_t, c_double, c_double) {
        let (tx, rx) = channel::<CTick>();

        // initialize the thread that blocks waiting for ticks
        thread::spawn(move ||{
            let mut rx_closure = get_rx_closure(dst);
            for t in rx.iter() {
                rx_closure(t);
            }
        });

        let username   = CString::new(CONF.fxcm_username).unwrap();
        let password   = CString::new(CONF.fxcm_password).unwrap();
        let url        = CString::new(CONF.fxcm_url).unwrap();
        let symbol     = CString::new(symbol).unwrap();
        let start_time = CString::new(start_time).unwrap();
        let end_time   = CString::new(end_time).unwrap();
        unsafe {
            let session_ptr = fxcm_login(username.as_ptr(), password.as_ptr(), url.as_ptr(), false);
            let tx_ptr = &tx as *const _ as *mut c_void;

            init_history_download(
                session_ptr,
                symbol.as_ptr(),
                start_time.as_ptr(),
                end_time.as_ptr(),
                Some(handler),
                tx_ptr
            );
        }
    }
}

extern fn handler(tx_ptr: *mut c_void, timestamp: uint64_t, bid: c_double, ask: c_double) {
    let sender: &Sender<CTick> = unsafe { transmute(tx_ptr) };
    let _ = sender.send( CTick{
        timestamp: timestamp,
        bid: bid,
        ask: ask
    });
}

pub fn get_rx_closure(dst: DataDst) -> RxCallback {
    match dst {
        DataDst::Console => {
            let inner = |t: CTick| {
                println!("{:?}", t);
            };

            RxCallback{
                inner: Box::new(inner),
            }
        },
        DataDst::Redis{host, channel} => {
            let client = get_client(host.as_str());
            let inner = move |ct: CTick| {
                let client = &client;
                // let tick_string = serde_json::to_string(&t).unwrap();
                // redis::cmd("PUBLISH")
                //     .arg(channel.clone())
                //     .arg(tick_string)
                //     .execute(client);
            };

            RxCallback{
                inner: Box::new(inner),
            }
        },
        _ => unimplemented!(),
    }
}

pub struct RxCallback {
    inner: Box<FnMut(CTick)>,
}

impl FnOnce<(CTick,)> for RxCallback {
    type Output = ();
    extern "rust-call" fn call_once(self, args: (CTick,)) {
        let mut inner = self.inner;
        inner(args.0)
    }
}

impl FnMut<(CTick,)> for RxCallback {
    extern "rust-call" fn call_mut(&mut self, args: (CTick,)) {
        (*self.inner)(args.0)
    }
}

struct TxCallback {
    inner: Box<FnMut(uint64_t, c_double, c_double)>,
}

impl FnOnce<(uint64_t, c_double, c_double,)> for TxCallback {
    type Output = ();
    extern "rust-call" fn call_once(self, args: (uint64_t, c_double, c_double,)) {
        let mut inner = self.inner;
        inner(args.0, args.1, args.2)
    }
}

impl FnMut<(uint64_t, c_double, c_double,)> for TxCallback {
    extern "rust-call" fn call_mut(&mut self, args: (uint64_t, c_double, c_double,)) {
        (*self.inner)(args.0, args.1, args.2)
    }
}

fn main() {
    let symbol = "EUR/USD";
    // m.d.Y H:M:S
    let start_time = "01.01.2012 00:00:00";
    let end_time   = "12.06.2016 00.00.00";
    let dst = DataDst::Console;

    let mut downloader = DataDownloader::new();
    downloader.init_download::<TxCallback>(symbol, dst, start_time, end_time);
}

#[test]
fn name() {
    unimplemented!();
}
