//! Code shared by all modules of the platform

#![feature(rustc_attrs, plugin, conservative_impl_trait, test)]

extern crate redis;
extern crate futures;
extern crate uuid;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate postgres;
extern crate test;
#[macro_use]
extern crate from_hashmap;

pub mod transport;
pub mod strategies;
pub mod trading;
pub mod conf;
