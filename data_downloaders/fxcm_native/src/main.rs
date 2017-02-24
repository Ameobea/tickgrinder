//! FXCM Native Data Downloader
//!
//! See README.txt for more information

#![feature(custom_derive, plugin, libc, conservative_impl_trait, unboxed_closures)]

extern crate libc;
extern crate tickgrinder_util;
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
use std::str::FromStr;

use uuid::Uuid;
use libc::{c_char, c_void, uint64_t, c_double, c_int};
use futures::stream::Stream;

use tickgrinder_util::transport::commands::*;
use tickgrinder_util::transport::redis::get_client as get_redis_client;
use tickgrinder_util::transport::redis::sub_multiple;
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::data::{transfer_data, get_rx_closure, TxCallback};
use tickgrinder_util::trading::tick::*;
use tickgrinder_util::conf::CONF;

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
    fn fxcm_login(
        username: *const c_char,
        password: *const c_char,
        url: *const c_char,
        live: bool,
        log_cb: Option<extern fn (env_ptr: *mut c_void, msg: *mut c_char, severity: CLogLevel)>,
        log_cb_env: *mut c_void
    ) -> *mut c_void;
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

const NULL: *mut c_void = 0 as *mut c_void;

// TODO: Move to Util
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
            timestamp: self.timestamp as u64,
            bid: bid_pips as usize,
            ask: ask_pips as usize,
        }
    }
}

#[derive(Serialize, PartialEq, Clone)]
struct DownloadDescriptor {
    symbol: String,
    start_time: u64,
    cur_time: u64,
    end_time: u64,
    dst: HistTickDst,
}

struct DataDownloader {
    uuid: Uuid,
    cs: CommandServer,
    running_downloads: Arc<Mutex<Vec<DownloadDescriptor>>>,
}

impl DataDownloader {
    pub fn new(uuid: Uuid) -> DataDownloader {
        DataDownloader {
            cs: CommandServer::new(uuid, "FXCM Native Data Downloader"),
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
                    let mut cs = self.cs.clone();
                    let our_instance = Instance {
                        uuid: self.uuid,
                        instance_type: String::from("FXCM Native Data Downloader"),
                    };
                    thread::spawn(move || {
                        let res = DataDownloader::init_download::<TxCallback>(
                            our_instance, symbol.as_str(), dst, start_time, end_time, running_downloads, &mut cs
                        );
                        println!("Results of download: {:?}", res);
                    });
                    Response::Ok
                },
                Command::ListRunningDownloads => self.list_running_downloads(),
                Command::GetDownloadProgress{id} => {
                    // just create a dummy result because we don't actually support checking progress
                    Response::DownloadProgress {
                        id: id,
                        start_time: 0,
                        end_time: 1,
                        cur_time: 0,
                    }
                },
                Command::CancelDataDownload{download_id: _} => {
                    Response::Error{
                        status: String::from("The FXCM Native Data Downloader doesn't support cancelling downloads."),
                    }
                },
                Command::TransferHistData{src, dst} => {
                    transfer_data(src, dst, self.cs.clone());
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
        downloader: Instance, symbol: &str, dst: HistTickDst, start_time: u64, end_time: u64,
        running_downloads: Arc<Mutex<Vec<DownloadDescriptor>>>, mut cs: &mut CommandServer
    ) -> Result<(), String> where F: FnMut(uint64_t, c_double, c_double) {
        let (tx, rx) = channel::<CTick>();

        // create these now before we convert our arguments into CStrings
        let descriptor = DownloadDescriptor {
            symbol: symbol.to_string(),
            start_time: start_time,
            cur_time: start_time, // :shrug:
            end_time: end_time,
            dst: dst.clone(),
        };

        let download_id = Uuid::new_v4();

        // command to be broadcast after download is complete indicating its completion.
        let done_cmd = Command::DownloadComplete{
            id: download_id,
            downloader: downloader.clone(),

            start_time: start_time,
            end_time: end_time,
            symbol: symbol.to_string(),
            dst: dst.clone(),
        };

        // get the digit count after the decimal for tick conversion
        let c_symbol     = CString::new(symbol).unwrap();
        let username   = CString::new(CONF.fxcm_username).unwrap();
        let password   = CString::new(CONF.fxcm_password).unwrap();
        let url        = CString::new(CONF.fxcm_url).unwrap();

        let session_ptr: *mut c_void;
        let digit_count: usize;
        unsafe{
            session_ptr = fxcm_login(username.as_ptr(), password.as_ptr(), url.as_ptr(), false, None, NULL);
            if session_ptr.is_null() {
                return Err(String::from("External login function returned nullptr; FXCM servers are likely down."))
            }
            let offer_row = get_offer_row(session_ptr, c_symbol.as_ptr());
            digit_count = getDigits(offer_row) as usize;
        }

        // try to get a closure here to make sure it works before spawning thread.
        // have to do twice since closures aren't +Send
        let _ = try!(get_rx_closure(dst.clone()));

        // initialize the thread that blocks waiting for ticks
        let dst_clone = dst.clone();
        thread::spawn(move ||{
            let mut rx_closure = get_rx_closure(dst_clone).unwrap();

            for ct in rx.iter() {
                let t: Tick = ct.to_tick(digit_count);
                rx_closure(t);
            }
        });

        {
            let mut running_downloads = running_downloads.lock().unwrap();
            running_downloads.push(descriptor.clone())
        }

        let c_start_time = CString::new(start_time.to_string()).unwrap();
        let c_end_time   = CString::new(end_time.to_string()).unwrap();

        // notify the platform that the download has started
        cs.send_forget(&Command::DownloadStarted {
            id: download_id,
            downloader: downloader,
            start_time: start_time,
            end_time: end_time,
            symbol: String::from(symbol),
            dst: dst,
        }, CONF.redis_control_channel);

        unsafe {
            let tx_ptr = &tx as *const _ as *mut c_void;

            init_history_download(
                session_ptr,
                c_symbol.as_ptr(),
                c_start_time.as_ptr(),
                c_end_time.as_ptr(),
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

/// A function passed off as a tick callback to the native C++ application.
#[no_mangle]
pub extern fn handler(tx_ptr: *mut c_void, timestamp: uint64_t, bid: c_double, ask: c_double) {
    let sender: &Sender<CTick> = unsafe { &*(tx_ptr as *const std::sync::mpsc::Sender<CTick>) };
    let _ = sender.send( CTick{
        timestamp: timestamp,
        bid: bid,
        ask: ask
    });
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
    use tickgrinder_util::transport::redis::sub_channel;
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
        let res = DataDownloader::init_download::<TxCallback>(symbol, dst, start_time, end_time, downloader.running_downloads.clone(), downloader.cs.clone());
        assert!(res.is_ok());
    });

    let rx = sub_channel(CONF.redis_host, channel_str);
    let responses: Vec<Result<String, ()>> = rx.wait().take(50).collect();
    assert_eq!(responses.len(), 50);
}
