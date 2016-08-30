//! A set of functions for interfacing with the tick processor
//!
//! These create an abstraction layer of things such as passing messages over
//! redis, creating/managing conditions, and managing tick processor instances.
//!
//! Commands are sent one at a time to the bot

use transport::command_server;
