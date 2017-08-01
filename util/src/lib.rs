//! Code shared by all modules of the platform

#![feature(rustc_attrs, plugin, conservative_impl_trait, test, fn_traits, core, unboxed_closures, libc, raw)]

#![allow(unknown_lints)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate redis;
extern crate futures;
extern crate uuid;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate postgres;
extern crate csv;
extern crate rand;
extern crate time;
extern crate test;
extern crate libc;

pub mod transport;
pub mod strategies;
pub mod trading;
pub mod instance;
pub mod conf;
