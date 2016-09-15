//! Utility functions for use by all parts of the algobot trading system

#![feature(custom_derive, plugin, conservative_impl_trait, test)]
#![plugin(serde_macros)]

extern crate redis;
extern crate futures;
extern crate uuid;
extern crate serde;
extern crate serde_json;
extern crate postgres;
extern crate test;

pub mod transport;
pub mod tick;
pub mod strategies;
