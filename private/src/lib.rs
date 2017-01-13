//! Private data directory for the platform.  See README.md for more information.

#![feature(test, plugin, custom_derive, conservative_impl_trait)]

extern crate test;
extern crate postgres;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate tickgrinder_util;
extern crate futures;
extern crate fxcm;

pub mod indicators;
pub mod trading_conditions;
pub mod strategies;

// Sets up the defaults for your application
pub use fxcm::FXCMNative as ActiveBroker;
