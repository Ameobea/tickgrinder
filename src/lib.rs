//! Utility functions for use by all parts of the algobot trading system

#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate redis;
extern crate futures;
extern crate uuid;
extern crate serde;
extern crate serde_json;
extern crate postgres;

pub mod transport;
pub mod tick;
pub mod strategies;
