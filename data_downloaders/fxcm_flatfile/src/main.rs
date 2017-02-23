//! A data downloader to download the flatfile historical tick archives hosted by FXCM

#![feature(rustc_attrs, conservative_impl_trait, associated_consts, custom_derive, slice_patterns)]

extern crate uuid;
extern crate flate2;
extern crate hyper;
extern crate tickgrinder_util;
extern crate tempdir;

use std::env;
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::Path;

use uuid::Uuid;

use tickgrinder_util::instance::PlatformInstance;
use tickgrinder_util::transport::commands::{Command, Response, Instance, HistTickDst};
use tickgrinder_util::transport::command_server::CommandServer;

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

/// Represents an in-progress data download.
struct RunningDownload {

}

struct Downloader {
    us: Instance, // our internal representation as an instance
    cs: CommandServer,
    running_downloads: Arc<Mutex<HashMap<Uuid, RunningDownload>>>,
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
        }
    }

    /// Starts a download of historical ticks
    pub fn init_download(&mut self, start_time: u64, end_time: u64, symbol: String, dst: HistTickDst) -> Response {
        if !SUPPORTED_PAIRS.contains(&symbol.to_uppercase().trim()) {
            return Response::Error{status: format!("The FXCM Flatfile Data Downloader does not support the symbol {}", symbol)};
        }

        unimplemented!();
    }

    /// Downloads a file using HTTP and saves it to the supplied path.
    fn download_file(&mut self, url: &str, dst: &Path) {
        unimplemented!(); // TODO

    }

    /// Decompresses a gzip-encoded file, writing the output to `dst`.
    fn gunzip(&mut self, src: &Path, dst: &Path) {
        unimplemented!(); // TODO
    }
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
