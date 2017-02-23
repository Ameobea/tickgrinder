//! A data downloader to download the flatfile historical tick archives hosted by FXCM

#![feature(rustc_attrs, conservative_impl_trait, associated_consts, custom_derive, slice_patterns)]

extern crate uuid;
extern crate libflate;
extern crate hyper;
extern crate chrono;
extern crate tickgrinder_util;
#[macro_use]
extern crate lazy_static;
extern crate tempdir;

use std::env;
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::Path;
use std::io::{Read, Write};
use std::fmt::Debug;
use std::fs::File;

use uuid::Uuid;
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use hyper::client::Client;
use libflate::gzip::Decoder;
use tempdir::TempDir;

use tickgrinder_util::instance::PlatformInstance;
use tickgrinder_util::transport::commands::{Command, Response, Instance, HistTickDst};
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::conf::CONF;

const NAME: &'static str = "FXCM Flatfile Data Downloader";

/// List of all currency pairs that can be downloaded using this tool
const SUPPORTED_PAIRS: &'static [&'static str] = &[
    "AUDCAD", "AUDCHF", "AUDJPY",
    "AUDNZD", "CADCHF", "EURAUD",
    "EURCHF", "EURGBP", "EURJPY",
    "EURUSD", "GBPCHF", "GBPJPY",
    "GBPNZD", "GBPUSD", "NZDCAD",
    "NZDCHF", "NZDJPY", "NZDUSD",
    "USDCAD", "USDCHF", "USDJPY",
];

/// the earliest supported start data of historical data
lazy_static!{
    static ref DATA_START: NaiveDateTime = NaiveDate::from_ymd(2015, 01, 04).and_hms(00, 00, 00);
}

/// debug-formats anything and returns the resulting `String`
fn debug<T: Debug>(x: T) -> String {
    format!("{:?}", x)
}

/// Returns the download link to a piece of compressed tick data for a given symbol and a given month.
fn get_data_url(symbol: &str, year: i32, month: u32) -> String {
    format!("https://tickdata.fxcorporate.com/{}/{}/{}.csv.gz", symbol, year, month)
}

/// Represents an in-progress data download.
struct RunningDownload {

}

#[derive(Clone)]
struct Downloader {
    us: Instance, // our internal representation as an instance
    cs: CommandServer,
    running_downloads: Arc<Mutex<HashMap<Uuid, RunningDownload>>>,
    http_client: Arc<Client>,
}

impl PlatformInstance for Downloader {
    fn handle_command(&mut self, cmd: Command) -> Option<Response> {
        match cmd {
            Command::Ping => Some(Response::Pong{ args: vec![self.us.uuid.hyphenated().to_string()] }),
            Command::Type => Some(Response::Info{ info: String::from(NAME) }),
            Command::Kill => {
                thread::spawn(|| {
                    thread::sleep(std::time::Duration::from_secs(3));
                    println!("the End is near...");
                    std::process::exit(0);
                });

                Some(Response::Info{info: format!("{} will terminate in 3 seconds.", NAME)})
            },
            Command::DownloadTicks{start_time, end_time, symbol, dst} => Some(self.init_download(start_time, end_time, symbol, dst)),
            _ => None,
        }
    }
}

impl Downloader {
    pub fn new(uuid: Uuid) -> Downloader {
        let cs = CommandServer::new(uuid, NAME);
        Downloader {
            us: Instance {
                instance_type: String::from(NAME),
                uuid: uuid,
            },
            cs: cs,
            running_downloads: Arc::new(Mutex::new(HashMap::new())),
            http_client: Arc::new(Client::new()),
        }
    }

    /// Starts a download of historical ticks
    pub fn init_download(&mut self, start_time: u64, end_time: u64, symbol: String, dst: HistTickDst) -> Response {
        let download_id = Uuid::new_v4();
        let symbol: String = symbol.trim().to_uppercase();
        if !SUPPORTED_PAIRS.contains(&symbol.as_str()) {
            return Response::Error{status: format!("The FXCM Flatfile Data Downloader does not support the symbol {}", symbol)};
        }

        // get the starting month and year of the data download
        let secs = ((start_time / 1000) / 1000) / 1000; // convert ns into seconds
        let mut naive = NaiveDateTime::from_timestamp(secs as i64, 0);
        if naive < *DATA_START {
            naive = *DATA_START;
        }
        let mut year = naive.year();
        let mut week = (naive.day() / 7) + 1; // gets current week of the year starting at 1

        // start the data download in another thread
        let mut clone = self.clone();
        thread::spawn(move || {
            let dst_dir = TempDir::new(&symbol).expect("Unable to create temporary directory");
            loop {
                let download_url = get_data_url(&symbol, year, week);
                let dst_path = &dst_dir.path().join(&format!("{}.csv", week));

                match download_chunk(&*clone.http_client, &download_url, dst_path) {
                    Ok(true) => {
                        if week < 52 { week += 1; } else {
                            week = 1;
                            year += 1;
                        }
                    },
                    Ok(false) => { // download is complete
                        // write the data into the `HistTickDst`
                        unimplemented!(); // TODO

                        // send `DownloadComplete` message to the platform
                        let cmd = Command::DownloadComplete {
                            id: download_id,
                            downloader: clone.us.clone(),
                            start_time: start_time,
                            end_time: end_time,
                            symbol: symbol,
                            dst: dst,
                        };
                        clone.cs.send_forget(&cmd, CONF.redis_control_channel);
                        break;
                    },
                    Err(err) => {
                        clone.cs.error(Some("HTTP"), &format!("Error during HTTP request to download {}: {}", download_url, err));
                        break;
                    }
                }
            }
        });

        unimplemented!();
    }
}

/// Downloads a file using HTTP, decompresses it using GZIP, and saves it to the supplied path.  The return value of the boolean
/// is true if the download was successful and false if it was a 404 error.
fn download_chunk(http_client: &Client, url: &str, dst: &Path) -> Result<bool, String> {
    // make the HTTP request and make sure it was successful
    let res = http_client.get(url).send().expect(&format!("Error while sending HTTP request to {}", url));
    if res.status == hyper::NotFound {
        return Ok(false);
    } else if res.status != hyper::Ok {
        return Err(format!("Unexpected response type from HTTP request: {:?}", res.status));
    }

    // create a new Gzip decoder to decompress the data from the HTTP response
    let mut decoder = try!(Decoder::new(res).map_err(debug));
    // allocate a 1MB buffer for the data from the unzipped data
    let mut buf = Box::new([0u8; 1024 * 1024]);
    // create the output file
    let mut dst_file = File::create(dst).map_err(debug)?;

    // keep reading chunks of data out of the decoder until it's empty and writing them to file
    loop {
        let bytes_read = decoder.read(buf.as_mut()).map_err(debug)?;
        // we're done if we read 0 bytes
        if bytes_read == 0 {
            break;
        }

        // write the read bytes into the destination file
        dst_file.write(&buf.as_ref()[0..bytes_read]).map_err(debug)?;
    }

    dst_file.sync_all().map_err(|_| format!("Unable to sync file to disc: {:?}", dst_file))?;
    Ok(true)
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let uuid: Uuid;

    match *args.as_slice() {
        [_, ref uuid_str] => {
            uuid = Uuid::parse_str(uuid_str.as_str())
                .expect("Unable to parse Uuid from supplied argument");
        },
        _ => panic!("Wrong number of arguments provided!  Usage: ./tick_processor [uuid] [symbol]"),
    }

    let downloader = Downloader::new(uuid);
    let mut csc = downloader.cs.clone();
    downloader.listen(uuid, &mut csc);
}

/// Make sure that our day-of-year to week-of-year conversion works correctly
#[test]
fn day_to_week() {
    let mut year  = 2014;
    let mut month = 3;
    let mut dom   = 12;
    assert_eq!((naive.day() / 7) + 1, 2);
}
