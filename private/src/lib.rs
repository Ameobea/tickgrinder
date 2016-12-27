//! Private data directory for the platform.  See README.md for more information.

#![feature(test, plugin, proc_macro, custom_derive)]

extern crate test;
extern crate postgres;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate algobot_util;

pub mod indicators;
