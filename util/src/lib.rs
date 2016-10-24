//! Utility functions for use by all parts of the algobot trading system

#![feature(custom_derive, plugin, conservative_impl_trait, test)]
#![feature(proc_macro)]

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
pub mod tick;
pub mod strategies;
