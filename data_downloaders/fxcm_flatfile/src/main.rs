//! A data downloader to download the flatfile historical tick archives hosted by FXCM

#![feature(rustc_attrs, conservative_impl_trait, associated_consts, custom_derive, test, slice_patterns)]

extern crate uuid;
extern crate tickgrinder_util;

use std::env;
use uuid::Uuid;

use tickgrinder_util::instance::PlatformInstance;
use tickgrinder_util::transport::commands::{Command, Response, Instance};
use tickgrinder_util::transport::command_server::CommandServer;

struct Downloader {
    us: Instance, // our internal representation as an instance
    cs: CommandServer,
}

impl PlatformInstance for Downloader {
    fn handle_command(&mut self, cmd: Command) -> Option<Response> {
        // TODO
        unimplemented!();
    }
}

impl Downloader {
    fn new(uuid: Uuid) -> Downloader {
        let cs = CommandServer::new(uuid, "FXCM Flatfile Data Downloader");
        Downloader {
            us: Instance {
                instance_type: String::from("FXCM Flatfile Data Downloader"),
                uuid: uuid,
            },
            cs: cs,
        }
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
