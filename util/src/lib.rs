//! Code shared by all modules of the platform

#![feature(custom_derive, plugin, conservative_impl_trait, test, proc_macro)]

extern crate redis;
extern crate futures;
extern crate uuid;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate postgres;
extern crate test;

pub mod transport;
pub mod strategies;
pub mod trading;
