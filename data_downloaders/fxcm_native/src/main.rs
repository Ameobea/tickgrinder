//! FXCM Native Data Downloader
//!
//! See README.txt for more information

#![feature(custom_derive, plugin, proc_macro, libc, conservative_impl_trait, fn_traits, unboxed_closures)]

extern crate libc;
extern crate algobot_util;
extern crate redis;
extern crate time;
extern crate futures;
extern crate postgres;
extern crate uuid;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::ffi::CString;
use std::mem::transmute;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::prelude::*;
use std::fmt;

use uuid::Uuid;
use libc::{c_char, c_void, uint64_t, c_double, c_int};
use futures::stream::Stream;

use algobot_util::transport::commands::*;
use algobot_util::transport::redis::get_client as get_redis_client;
use algobot_util::transport::redis::sub_multiple;
use algobot_util::transport::postgres::*;
use algobot_util::transport::query_server::QueryServer;
use algobot_util::transport::command_server::{CommandServer, CsSettings};
use algobot_util::trading::tick::*;
use algobot_util::conf::CONF;

mod util;
use util::transfer_data;

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

pub const PG_CONF: PostgresConf = PostgresConf {
    postgres_user: CONF.postgres_user,
    postgres_db: CONF.postgres_db,
    postgres_password: CONF.postgres_password,
    postgres_port: CONF.postgres_port,
    postgres_url: CONF.postgres_host,
};

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

#[derive(Serialize, PartialEq, Clone)]
struct DownloadDescriptor {
    symbol: String,
    start_time: String,
    end_time: String,
    dst: HistTickDst,
}

struct DataDownloader {
    uuid: Uuid,
    cs: CommandServer,
    running_downloads: Arc<Mutex<Vec<DownloadDescriptor>>>,
}

impl DataDownloader {
    pub fn new(uuid: Uuid) -> DataDownloader {
        let css = CsSettings {
            conn_count: 5,
            max_retries: 3,
            redis_host: CONF.redis_host,
            responses_channel: CONF.redis_responses_channel,
            timeout: 300,
        };

        DataDownloader {
            cs: CommandServer::new(css),
            uuid: uuid,
            running_downloads: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Start listening for commands and responding to them
    pub fn listen(&mut self) {
        let client = get_redis_client(CONF.redis_host);
        let cmd_rx = sub_multiple(CONF.redis_host, &[self.uuid.hyphenated().to_string().as_str(), CONF.redis_control_channel]);
        send_command(&Command::Ready{
            instance_type: "FXCM Native Data Downloader".to_string(),
            uuid: self.uuid
        }.wrap(), &client, CONF.redis_control_channel)
            .expect("Unable to send Ready command over Redis.");

        for res in cmd_rx.wait() {
            let (_, wr_cmd_string) = res.unwrap();
            let wr_cmd_res = WrappedCommand::from_str(wr_cmd_string.as_str());
            if wr_cmd_res.is_err() {
                println!("Unable to parse {} into WrappedCommand", wr_cmd_string);
            }
            let wr_cmd = wr_cmd_res.unwrap();

            let res = match wr_cmd.cmd {
                Command::Ping => Response::Pong{ args: vec![self.uuid.hyphenated().to_string()] },
                Command::Type => Response::Info{ info: "FXCM Native Data Downloader".to_string() },
                Command::DownloadTicks{start_time, end_time, symbol, dst} => {
                    let running_downloads = self.running_downloads.clone();
                    let cs = self.cs.clone();
                    thread::spawn(move || {
                        let res = DataDownloader::init_download::<TxCallback>(
                            symbol.as_str(), dst, start_time.as_str(), end_time.as_str(), running_downloads, cs
                        );
                        println!("Results of download: {:?}", res);
                    });
                    Response::Ok
                },
                Command::ListRunningDownloads => self.list_running_downloads(),
                Command::TransferHistData{src, dst} => {
                    transfer_data(src, dst);
                    Response::Ok
                },
                Command::Kill => {
                    thread::spawn(|| {
                        thread::sleep(std::time::Duration::from_secs(3));
                        std::process::exit(0);
                    });

                    Response::Info{info: "Data Downloader shutting down in 3 seconds...".to_string()}
                },
                _ => Response::Error{ status: "Data Downloader doesn't recognize that command.".to_string() },
            };
            let wr_res = res.wrap(wr_cmd.uuid);
            let _ = send_response(&wr_res, &client, CONF.redis_responses_channel);
        }
    }

    pub fn init_download<F>(
        symbol: &str, dst: HistTickDst, start_time: &str, end_time: &str, running_downloads: Arc<Mutex<Vec<DownloadDescriptor>>>, mut cs: CommandServer
    ) -> Result<(), String> where F: FnMut(uint64_t, c_double, c_double) {
        let (tx, rx) = channel::<CTick>();

        // create these now before we convert our arguments into CStrings
        let descriptor = DownloadDescriptor {
            symbol: symbol.to_string(),
            start_time: start_time.to_string(),
            end_time: end_time.to_string(),
            dst: dst.clone(),
        };

        // command to be broadcast after download is complete indicating its completion.
        let done_cmd = Command::DownloadComplete{
            start_time: start_time.to_string(),
            end_time: end_time.to_string(),
            symbol: symbol.to_string(),
            dst: dst.clone(),
        };

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

        {
            let mut running_downloads = running_downloads.lock().unwrap();
            running_downloads.push(descriptor.clone())
        }

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

        // remove descriptor from list once download is finished.
        {
            let mut running_downloads = running_downloads.lock().unwrap();
            for i in 0..running_downloads.len() {
                if running_downloads[i] == descriptor {
                    running_downloads.remove(i);
                    break;
                }
            }
        }

        // send command indicating download completion
        let _ = cs.execute(done_cmd, CONF.redis_control_channel.to_string());

        Ok(())
    }

    /// Returns a list of running downloads
    pub fn list_running_downloads(&self) -> Response {
        let running = self.running_downloads.lock().unwrap();
        let res = serde_json::to_string(&*running).unwrap();
        Response::Info{info: res}
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

pub fn get_rx_closure(dst: HistTickDst) -> Result<RxCallback, String> {
    let cb = match dst.clone() {
        HistTickDst::Console => {
            let inner = |t: Tick| {
                println!("{:?}", t);
            };

            RxCallback{
                dst: dst,
                inner: Box::new(inner),
            }
        },
        HistTickDst::RedisChannel{host, channel} => {
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
        HistTickDst::RedisSet{host, set_name} => {
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
        HistTickDst::Flatfile{filename} => {
            let fnc = filename.clone();
            let path = Path::new(&fnc);
            // create the file if it doesn't exist
            if !path.exists() {
                let _ = File::create(path).unwrap();
            }

            // try to open the specified filename in append mode
            let file_opt = OpenOptions::new().append(true).open(path);
            if file_opt.is_err() {
                return Err(format!("Unable to open file with path {}", filename));
            }
            let mut file = file_opt.unwrap();

            let inner = move |t: Tick| {
                let tick_string = t.to_csv_row();
                file.write_all(tick_string.as_str().as_bytes())
                    .expect(format!("couldn't write to output file: {}, {}", filename, tick_string).as_str());
            };

            RxCallback {
                dst: dst,
                inner: Box::new(inner),
            }
        },
        HistTickDst::Postgres{table} => {
            let connection_opt = get_client(PG_CONF);
            if connection_opt.is_err() {
                return Err(String::from("Unable to connect to PostgreSQL!"))
            }
            let connection = connection_opt.unwrap();
            let _ = try!(init_hist_data_table(table.as_str(), &connection, CONF.postgres_user));
            let mut qs = QueryServer::new(10, PG_CONF);

            let mut inner_buffer = Vec::with_capacity(5000);

            let inner = move |t: Tick| {
                let val = format!("({}, {}, {})", t.timestamp, t.bid, t.ask);
                inner_buffer.push(val);
                if inner_buffer.len() > 4999 {
                    let mut query = String::from(format!("INSERT INTO {} (tick_time, bid, ask) VALUES ", table));
                    let values = inner_buffer.as_slice().join(", ");
                    query += &values;
                    query += ";";

                    qs.execute(query);
                    inner_buffer.clear();
                }
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
    dst: HistTickDst,
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
    // ./fxcm_native uuid
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        panic!("Usage: ./fxcm_native uuid");
    }

    let uuid = Uuid::parse_str(args[1].as_str())
        .expect("Unable to parse Uuid from supplied argument");
    let mut dd = DataDownloader::new(uuid);
    dd.listen();
}

#[test]
fn history_downloader_functionality() {
    use algobot_util::transport::redis::sub_channel;
    let start_time = "01.01.2012 00:00:00";
    let end_time   = "12.06.2016 00.00.00";
    let symbol = "EUR/USD";

    let channel_str = "TEST_fxcm_dd_native";
    let dst = HistTickDst::RedisChannel{
        host: CONF.redis_host.to_string(),
        channel: channel_str.to_string(),
    };

    // start data download in another thread as to not block
    thread::spawn(move ||{
        let downloader = DataDownloader::new(Uuid::new_v4());
        let _ = DataDownloader::init_download::<TxCallback>(symbol, dst, start_time, end_time, downloader.running_downloads.clone(), downloader.cs.clone());
    });

    let rx = sub_channel(CONF.redis_host, channel_str);
    let responses: Vec<Result<String, ()>> = rx.wait().take(50).collect();
    assert_eq!(responses.len(), 50);
}
