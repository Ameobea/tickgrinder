//! FXCM Native Data Downloader
//!
//! See README.txt for more information

#![feature(libc, conservative_impl_trait, fn_traits, unboxed_closures)]

extern crate libc;
extern crate algobot_util;
extern crate redis;
extern crate serde_json;
extern crate time;
extern crate futures;
extern crate postgres;

use std::thread;
use std::sync::mpsc::{channel, Sender};
use std::ffi::CString;
use std::mem::transmute;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::fmt;

use libc::{c_char, c_void, uint64_t, c_double, c_int};

use algobot_util::transport::redis::get_client as get_redis_client;
use algobot_util::transport::postgres::*;
use algobot_util::transport::query_server::QueryServer;
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

    fn get_offer_row(connection: *mut c_void, instrument: *const c_char) -> *mut c_void;
    fn getDigits(row: *mut c_void) -> c_int;
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
        let multiplier = 10usize.pow(decimals as u32) as f64;
        let bid_pips = self.bid * multiplier;
        let ask_pips = self.ask * multiplier;
        Tick {
            timestamp: self.timestamp as usize,
            bid: bid_pips as usize,
            ask: ask_pips as usize,
        }
    }
}

struct DataDownloader {}

/// Where to save the recorded ticks to.
#[derive(Debug, Clone)]
pub enum DataDst {
    Flatfile{filename: String},
    Postgres{table: String},
    RedisChannel{host: String, channel: String},
    RedisSet{host: String, set_name: String},
    Console,
}

impl DataDownloader {
    pub fn new() -> DataDownloader {
        DataDownloader {}
    }

    pub fn init_download<F>(
        &mut self, symbol: &str, dst: DataDst, start_time: &str, end_time: &str,
    ) -> Result<(), String> where F: FnMut(uint64_t, c_double, c_double) {
        let (tx, rx) = channel::<CTick>();

        // get the digit count after the decimal for tick conversion
        let symbol     = CString::new(symbol).unwrap();
        let username   = CString::new(CONF.fxcm_username).unwrap();
        let password   = CString::new(CONF.fxcm_password).unwrap();
        let url        = CString::new(CONF.fxcm_url).unwrap();

        let session_ptr: *mut c_void;
        let digit_count: usize;
        unsafe{
            session_ptr = fxcm_login(username.as_ptr(), password.as_ptr(), url.as_ptr(), false);
            let offer_row = get_offer_row(session_ptr, symbol.as_ptr());
            digit_count = getDigits(offer_row) as usize;
        }

        // try to get a closure here to make sure it works before spawning thread.
        // have to do twice since closures aren't +Send
        let _ = try!(get_rx_closure(dst.clone()));

        // initialize the thread that blocks waiting for ticks
        thread::spawn(move ||{
            let mut rx_closure = get_rx_closure(dst).unwrap();

            for ct in rx.iter() {
                let t: Tick = ct.to_tick(digit_count);
                rx_closure(t);
            }
        });

        let start_time = CString::new(start_time).unwrap();
        let end_time   = CString::new(end_time).unwrap();
        unsafe {
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

        Ok(())
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

pub fn get_rx_closure(dst: DataDst) -> Result<RxCallback, String> {
    let cb = match dst.clone() {
        DataDst::Console => {
            let inner = |t: Tick| {
                println!("{:?}", t);
            };

            RxCallback{
                dst: dst,
                inner: Box::new(inner),
            }
        },
        DataDst::RedisChannel{host, channel} => {
            let client = get_redis_client(host.as_str());
            // buffer up 5000 ticks in memory and send all at once to avoid issues
            // with persistant redis connections taking up lots of ports
            let mut buffer: Vec<String> = Vec::with_capacity(5000);

            let inner = move |t: Tick| {
                let client = &client;
                let tick_string = serde_json::to_string(&t).unwrap();
                buffer.push(tick_string);

                // Send all buffered ticks once the buffer is full
                if buffer.len() >= 5000 {
                    let mut pipe = redis::pipe();
                    for item in buffer.drain(..) {
                        pipe.cmd("PUBLISH")
                            .arg(&channel)
                            .arg(item);
                    }
                    pipe.execute(client);
                }
            };

            RxCallback {
                dst: dst,
                inner: Box::new(inner),
            }
        },
        DataDst::RedisSet{host, set_name} => {
            let client = get_redis_client(host.as_str());
            // buffer up 5000 ticks in memory and send all at once to avoid issues
            // with persistant redis connections taking up lots of ports
            let mut buffer: Vec<String> = Vec::with_capacity(5000);

            let inner = move |t: Tick| {
                let client = &client;
                let tick_string = serde_json::to_string(&t).unwrap();
                buffer.push(tick_string);

                // Send all buffered ticks once the buffer is full
                if buffer.len() >= 5000 {
                    let mut pipe = redis::pipe();
                    for item in buffer.drain(..) {
                        pipe.cmd("SADD")
                            .arg(&set_name)
                            .arg(item);
                    }
                    pipe.execute(client);
                }
            };

            RxCallback {
                dst: dst,
                inner: Box::new(inner),
            }
        },
        DataDst::Flatfile{filename} => {
            let file_opt = OpenOptions::new().append(true).open(filename.clone());
            if file_opt.is_err() {
                return Err(format!("Unable to open file with path {:?}", filename));
            }
            let mut file = file_opt.unwrap();

            let inner = move |t: Tick| {
                let tick_string = serde_json::to_string(&t).unwrap();
                file.write_all(tick_string.as_str().as_bytes())
                    .expect(format!("couldn't write to output file: {}, {}", filename, tick_string).as_str());
            };

            RxCallback {
                dst: dst,
                inner: Box::new(inner),
            }
        },
        DataDst::Postgres{table} => {
            let pg_conf = PostgresConf {
                postgres_user: CONF.postgres_user,
                postgres_db: CONF.postgres_db,
                postgres_password: CONF.postgres_password,
                postgres_port: CONF.postgres_port,
                postgres_url: CONF.postgres_url,
            };
            let connection_opt = get_client(pg_conf.clone());
            if connection_opt.is_err() {
                return Err(String::from("Unable to connect to PostgreSQL!"))
            }
            let connection = connection_opt.unwrap();
            let _ = try!(init_hist_data_table(table.as_str(), &connection, CONF.postgres_user));
            let mut qs = QueryServer::new(10, pg_conf);

            let inner = move |t: Tick| {
                t.store_table(table.as_str(), &mut qs);
            };

            RxCallback {
                dst: dst,
                inner: Box::new(inner),
            }
        },
    };

    return Ok(cb);
}

pub struct RxCallback {
    dst: DataDst,
    inner: Box<FnMut(Tick)>,
}

impl FnOnce<(Tick,)> for RxCallback {
    type Output = ();
    extern "rust-call" fn call_once(self, args: (Tick,)) {
        let mut inner = self.inner;
        inner(args.0)
    }
}

impl FnMut<(Tick,)> for RxCallback {
    extern "rust-call" fn call_mut(&mut self, args: (Tick,)) {
        (*self.inner)(args.0)
    }
}

impl fmt::Debug for RxCallback {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RxCallback: {:?}",  self.dst)
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
    let dst = DataDst::RedisSet{
        host: CONF.redis_host.to_string(),
        set_name: "TICKS_EURUSD".to_string()
    };

    let mut downloader = DataDownloader::new();
    let _ = downloader.init_download::<TxCallback>(symbol, dst, start_time, end_time);
}

/// Make sure that the C++ code calls the Rust function as a callback
#[test]
fn history_downloader_functionality() {
    use futures::stream::Stream;

    let start_time = "01.01.2012 00:00:00";
    let end_time   = "12.06.2016 00.00.00";
    let symbol = "EUR/USD";

    let channel_str = "TEST_fxcm_dd_native";
    let dst = DataDst::RedisChannel{
        host: CONF.redis_host.to_string(),
        channel: channel_str.to_string(),
    };

    // start data download in another thread as to not block
    thread::spawn(move ||{
        let mut downloader = DataDownloader::new();
        let _ = downloader.init_download::<TxCallback>(symbol, dst, start_time, end_time);
    });

    let rx = sub_channel(CONF.redis_host, channel_str);
    let responses: Vec<Result<String, ()>> = rx.wait().take(50).collect();
    println!("{:?}", responses);
    assert_eq!(responses.len(), 50);
}
