//! Defines some structs, enums, and constants mainly useful for passing information around
//! over the FFI and over threads.

use std::collections::HashMap;
use std::sync::Mutex;

use libc::{c_char, c_void, uint64_t, c_double, c_int};
use futures::sync::mpsc::{UnboundedSender, UnboundedReceiver};

use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::trading::broker::*;
use tickgrinder_util::trading::tick::*;

pub const NULL: *mut c_void = 0 as *mut c_void;

/// Contains all possible commands that can be received by the broker server.
#[repr(C)]
#[derive(Clone)]
#[allow(dead_code)]
pub enum ServerCommand {
    MARKET_OPEN,
    MARKET_CLOSE,
    LIST_ACCOUNTS,
    DISCONNECT,
    PING,
    INIT_TICK_SUB,
    GET_OFFER_ROW,
    DELETE_ORDER,
    MODIFY_ORDER,
}

/// Contains all possible responses that can be received by the broker server.
#[repr(C)]
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ServerResponse {
    POSITION_OPENED,
    POSITION_CLOSED,
    TRADE_EXECUTED,
    TRADE_CLOSED,
    SESSION_TERMINATED,
    PONG,
    ERROR,
    TICK_SUB_SUCCESSFUL,
    OFFER_ROW,
    ORDER_MODIFIED,
}

/// A packet of information asynchronously received from the broker server.
#[repr(C)]
#[derive(Clone)]
pub struct ServerMessage {
    pub response: ServerResponse,
    pub payload: *mut c_void,
}

/// A packet of information that can be sent to the broker server.
#[repr(C)]
#[derive(Clone)]
pub struct ClientMessage {
    pub command: ServerCommand,
    pub payload: *mut c_void,
}

pub struct FXCMNative {
    pub settings_hash: HashMap<String, String>,
    pub server_environment: *mut c_void,
    pub raw_rx: Option<UnboundedReceiver<BrokerResult>>,
    pub tickstream_obj: Mutex<Tickstream>,
}

// TODO: Move to Util
#[derive(Debug)]
#[repr(C)]
#[allow(dead_code)]
pub struct CTick {
    pub timestamp: uint64_t,
    pub bid: c_double,
    pub ask: c_double,
}

// TODO: Move to Util
#[derive(Debug)]
#[repr(C)]
pub struct CSymbolTick {
    pub symbol: *const c_char,
    pub timestamp: uint64_t,
    pub bid: c_double,
    pub ask: c_double,
}

impl CSymbolTick {
    /// Converts a CSymbolTick into a Tick given the amount of decimal places precision.
    pub fn to_tick(&self, decimals: usize) -> Tick {
        let multiplier = 10usize.pow(decimals as u32) as f64;
        let bid_pips = self.bid * multiplier;
        let ask_pips = self.ask * multiplier;

        Tick {
            timestamp: self.timestamp as usize,
            bid: bid_pips as usize,
            ask: ask_pips as usize,
        }
    }
}

/// Contains data necessary to initialize a tickstream
#[repr(C)]
pub struct TickstreamDef {
    pub env_ptr: *mut c_void,
    pub cb: Option<extern fn (tx_ptr: *mut c_void, cst: CSymbolTick)>,
}

/// Holds the currently subscribed symbols as well as a channel to send them through
pub struct Tickstream {
    pub subbed_pairs: Vec<SubbedPair>,
    pub cs: CommandServer,
}

/// A pair that a user has subscribed to containing the symbol, an sender through which to
/// send ticks, and the decimal precision of the exchange rate's float value.
pub struct SubbedPair {
    pub symbol: *const c_char,
    pub sender: UnboundedSender<Tick>,
    pub decimals: usize,
}

/// Holds the state for the `handle_message` function
pub struct HandlerState {
    pub sender: UnboundedSender<BrokerResult>,
    pub cs: CommandServer,
}

/// A request to open or close a position at market price.
#[repr(C)]
#[allow(dead_code)]
struct MarketRequest{
    pub symbol: *const c_char,
    pub quantity: c_int,
    // when opening a long or closing a short, should be true
    // when opening a short or closing a long, should be false
    pub is_long: bool,
    pub uuid: *const c_char,
}

// something to hold our environment so we can convince Rust to send it between threads
#[derive(Clone)]
pub struct Spaceship(pub *mut c_void);

unsafe impl Send for Spaceship{}
unsafe impl Send for FXCMNative {}
unsafe impl Send for ServerMessage {}
unsafe impl Send for ClientMessage {}
