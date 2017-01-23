//! Defines how the different modules of the platform talk to each other
//! and interact with the outside world.

pub mod redis;
pub mod postgres;
pub mod commands;
pub mod query_server;
pub mod command_server;
pub mod tickstream;
pub mod textlog;
