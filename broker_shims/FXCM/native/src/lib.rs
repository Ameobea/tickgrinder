//! Broker shim for FXCM.  Integrates directly with the C++ native FXCM API to map complete broker
//! functionality through into Rust.

#![feature(libc, test)]

extern crate uuid;
extern crate futures;
extern crate tickgrinder_util;
extern crate libc;
extern crate test;
extern crate redis;

use std::collections::HashMap;
use std::ffi::CString;
use std::ptr::drop_in_place;
use std::thread;
use std::slice;
use std::str;
use std::sync::Mutex;

use libc::{c_char, c_void, c_int, memchr};
use uuid::Uuid;
use futures::sync::oneshot;
use futures::stream::{Stream, BoxStream};
use futures::sync::oneshot::Receiver;
use futures::sync::mpsc::unbounded;

use tickgrinder_util::transport::commands::{LogLevel, CLogLevel};
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::redis::*;
use tickgrinder_util::trading::broker::*;
use tickgrinder_util::trading::tick::*;
use tickgrinder_util::trading::trading_condition::TradingAction;
use tickgrinder_util::conf::CONF;

mod helper_objects;
pub use helper_objects::FXCMNative;
use helper_objects::*;

// Link with all the FXCM native libraries, the C++ standard library, and the
// FXCM FFI library to expose complete functionality of external code
#[link(name="fxtp")]
#[link(name="gsexpat")]
#[link(name="gstool3")]
#[link(name="httplib")]
#[link(name="log4cplus")]
#[link(name="pdas")]
#[link(name="fxcm_ffi")]
#[link(name="stdc++")]
#[link(name="fxmsg")]
#[link(name="ForexConnect")]
#[link(name="sample_tools")]
extern {
    fn fxcm_login(
        username: *const c_char,
        password: *const c_char,
        url: *const c_char,
        live: bool,
        log_cb: Option<extern fn (env_ptr: *mut c_void, msg: *mut c_char, severity: CLogLevel)>,
        log_cb_env: *mut c_void
    ) -> *mut c_void;
    #[allow(dead_code)]
    fn test_login(username: *const c_char, password: *const c_char, url: *const c_char, live: bool) -> bool;

    // broker server commands
    fn init_server_environment(
        cb: Option<extern fn (tx_ptr: *mut c_void, message: *mut ServerMessage)>,
        tx_ptr: *mut c_void,
        log_cb: Option<extern fn (env_ptr: *mut c_void, msg: *mut c_char, severity: CLogLevel)>,
        log_cb_env: *mut c_void
    ) -> *mut c_void;
    fn start_server(session: *mut c_void, env: *mut c_void);
    fn push_client_message(message: ClientMessage, env: *mut c_void);
    fn getDigits(row: *mut c_void) -> c_int;
}

/// Deallocates the memory left behind from `into_raw()`ing a box.
#[allow(dead_code)]
fn free(ptr: *mut c_void) {
    unsafe { drop_in_place(ptr) };
}

/// Processes received messages from the broker server and converts them into `BrokerResult`s that can be fed to the
/// stream returned by `get_stream`.
extern fn handle_message(env_ptr: *mut c_void, message: *mut ServerMessage) {
    unsafe {
        let state = &mut *(env_ptr as *mut helper_objects::HandlerState);
        let sender = &mut state.sender;
        let timestamp = 0; // TODO: Use actual timestamps here
        let res: Option<BrokerResult> = match (*message).response {
            ServerResponse::POSITION_OPENED => {
                unimplemented!();
            },
            ServerResponse::PONG => {
                let micros: &u64 = &*((*message).payload as *const u64);
                let msg = BrokerMessage::Pong{time_received: *micros};
                Some(Ok(msg))
            },
            ServerResponse::ERROR => {
                let msg_ptr: *mut c_char = (*message).payload as *mut c_char;
                let err_msg: CString = ptr_to_cstring(msg_ptr);
                println!("DEBUG: {}", err_msg.to_str().unwrap());
                Some(Err(BrokerError::Message{message: err_msg.into_string()
                    .expect("Unable to convert CString ito String")}))
            },
            ServerResponse::OFFER_ROW => {
                let decimals: usize = getDigits((*message).payload) as usize;
                let client = get_client(CONF.redis_host);
                // send the decimal value down the magic pipe where we're waiting for it on the other end
                redis::cmd("PUBLISH")
                    .arg("redis_magic_pipe")
                    .arg(&decimals.to_string())
                    .execute(&client);
                None
            },
            ServerResponse::TICK_SUB_SUCCESSFUL => {
                state.cs.debug(None, "Broker server reports successful tick sub");
                None // TODO: eventually send message once we have IDs for the messages
            },
            _ => {
                let msg = format!("Received unhandlable result type from broker server: {:?}", (*message).response);
                println!("{}", msg);
                state.cs.error(None, &msg);
                unimplemented!()
            },
        };

        // libc::free((*message).payload);

        if res.is_some() {
            sender.send((timestamp, res.unwrap())).expect("Couldn't send message through sender in broker.");
        }
    }
}

extern fn log_cb(env_ptr: *mut c_void, msg: *mut c_char, severity: CLogLevel) {
    let mut cs: &mut CommandServer = unsafe { &mut *(env_ptr as *mut CommandServer) };
    let msg = unsafe { ptr_to_cstring(msg) };
    let loglevel = severity.convert();

    match msg.to_str() {
        Ok(msg_str) => {
            cs.log(None, msg_str, loglevel);
        },
        Err(err) => cs.log(None, &format!("Error when converting CString into &str: {}", err), LogLevel::Error),
    }
}

extern fn tick_cb(env_ptr: *mut c_void, cst: CSymbolTick) {
    let ts_amtx: &mut Mutex<Tickstream> = unsafe { &mut *(env_ptr as *mut Mutex<helper_objects::Tickstream>) };
    let mut ts = ts_amtx.lock().unwrap();

    for &mut SubbedPair{symbol, ref mut sender, decimals} in &mut ts.subbed_pairs {
        if unsafe { libc::strcmp(symbol, cst.symbol) } == 0 {
            // convert the CSymbolTick to a Tick using the stored decimal precision
            sender.send(cst.to_tick(decimals)).unwrap();
            return
        }
    }
}

/// Takes a pointer to a string from C and copies it into a Rust-owned `CString`.
unsafe fn ptr_to_cstring(ptr: *mut c_char) -> CString {
    // expect that no strings are longer than 100000 bytes
    let end_ptr = memchr(ptr as *const c_void, 0, 100000);
    let len: usize = end_ptr as usize - ptr as usize;
    let slice: &[u8] = slice::from_raw_parts(ptr as *const u8, len);
    CString::new(slice).expect("Unable to convert the slice into a CString")
}

impl Broker for FXCMNative {
    fn init(settings: HashMap<String, String>) -> Receiver<Result<Self, BrokerError>> where Self:Sized {
        let (ext_tx, ext_rx) = oneshot::channel::<Result<Self, BrokerError>>();
        thread::spawn(move || {
            // channel with which to receive messages from the server
            let (tx, rx) = unbounded::<(u64, BrokerResult)>();
            let instance_id = String::from("FXCM Native Broker");
            let cs = CommandServer::new(Uuid::new_v4(), &instance_id);
            let state = Box::new(HandlerState {
                sender: tx,
                cs: cs.clone(),
            });
            let handler_env_ptr = Box::into_raw(state) as *const _ as *mut c_void;

            let boxed_cs = Box::new(cs.clone());
            let env_ptr = Box::into_raw(boxed_cs) as *const _ as *mut c_void;
            let env_ptr_ship = Spaceship(env_ptr);

            let server_environment: *mut c_void = unsafe { init_server_environment(Some(handle_message), handler_env_ptr, Some(log_cb), env_ptr) };
            let ship = Spaceship(server_environment);

            thread::spawn(move || {
                match login(env_ptr_ship.0) {
                    Ok(session) => unsafe { start_server(session, ship.0) }, // blocks on C++ event loop
                    Err(msg) => println!("{}", msg),
                }
            });

            let obj = Mutex::new(Tickstream{subbed_pairs: Vec::new(), cs: cs});

            let inst = FXCMNative {
                settings_hash: settings,
                server_environment: server_environment,
                raw_rx: Some(Box::new(rx)),
                tickstream_obj: obj,
            };

            ext_tx.complete(Ok(inst));
        });

        ext_rx
    }

    fn list_accounts(&mut self) -> Receiver<Result<HashMap<Uuid, Account>, BrokerError>> {
        let msg = ClientMessage {
            command: ServerCommand::LIST_ACCOUNTS,
            payload: NULL,
        };

        unsafe { push_client_message(msg, self.server_environment) };

        unimplemented!();
    }

    fn get_ledger(&mut self, account_id: Uuid) -> Receiver<Result<Ledger, BrokerError>> {
        unimplemented!();
    }

    #[allow(unused_variables)]
    fn execute(&mut self, action: BrokerAction) -> PendingResult {
        match action {
            BrokerAction::Ping => {
                unimplemented!(); // TODO
            },
            BrokerAction::TradingAction{action, account_uuid} => {
                match action {
                    TradingAction::MarketOrder{symbol, long, size, stop, take_profit, max_range} => {
                        unimplemented!(); // TODO
                    },
                    TradingAction::ModifyOrder{uuid, size, entry_price, stop, take_profit} => {
                        unimplemented!(); // TODO
                    },
                    TradingAction::CancelOrder{uuid} => {
                        unimplemented!(); // TODO
                    },
                    TradingAction::MarketClose{uuid, size} => {
                        unimplemented!(); // TODO
                    },
                    TradingAction::LimitOrder{symbol, long, size, stop, take_profit, entry_price} => {
                        unimplemented!(); // TODO
                    },
                    TradingAction::LimitClose{uuid, size, exit_price} => {
                        unimplemented!(); // TODO
                    },
                    TradingAction::ModifyPosition{uuid, stop, take_profit} => {
                        unimplemented!(); // TODO
                    },
                }
            },
            BrokerAction::Disconnect => unimplemented!(),
        }
    }

    fn get_stream(&mut self) -> Result<BoxStream<(u64, BrokerResult), ()>, BrokerError> {
        if self.raw_rx.is_some() {
            Ok(self.raw_rx.take().unwrap())
        } else {
            Err(BrokerError::Message{message: String::from("The stream for this broker has already been taken.")})
        }
    }

    fn sub_ticks(&mut self, symbol: String) -> Result<Box<Stream<Item=Tick, Error=()> + Send>, BrokerError> {
        let (tx, rx) = unbounded::<Tick>();
        let mut cs: CommandServer;

        // create the Redis channel through which to receive the decimals
        let inner_rx = sub_channel(CONF.redis_host, "redis_magic_pipe");

        {
            let tickstream = self.tickstream_obj.lock().unwrap();
            cs = tickstream.cs.clone();
        }

        let symbol_cstr_res = CString::new(symbol.clone());
        if symbol_cstr_res.is_err() {
            let err = "Unable to convert symbol into CString!";
            cs.error(None, err);
            return Err(BrokerError::Message{message: String::from(err)})
        }
        let symbol_cstr = symbol_cstr_res.unwrap();

        let msg = ClientMessage {
            command: ServerCommand::GET_OFFER_ROW,
            payload: symbol_cstr.as_ptr() as *mut c_void,
        };

        // send the request to get the precision for that symbol
        unsafe { push_client_message(msg, self.server_environment) };

        // wait for the callback handler to send the message over redis
        let mut decimals: usize = 0;
        for msg in inner_rx.wait() {
            match msg.unwrap().parse::<usize>() {
                Ok(d) => {
                    decimals = d;
                    break;
                }
                Err(err) => {
                    let err_string = format!("Unable to convert the result from Redis into an int: {:?}", err);
                    println!("{}", err_string);

                    cs.critical(None, &err_string);
                    return Err(BrokerError::Message{message: err_string})
                }
            }
        }
        cs.debug(None, &format!("Received decimal precision for {}: {}", symbol, decimals));

        let cstring_symbol = CString::new(symbol.as_str()).unwrap();
        let subbed_pair = SubbedPair {
            // leave the string allocated on the heap and don't deallocate it
            symbol: cstring_symbol.into_raw() as *const c_char,
            sender: tx,
            decimals: decimals,
        };

        {
            let mut tickstream = self.tickstream_obj.lock().unwrap();
            tickstream.subbed_pairs.push(subbed_pair);
        }

        let def = Box::new(TickstreamDef {
            env_ptr: &self.tickstream_obj as *const _ as *mut c_void,
            cb: Some(tick_cb),
        });

        let msg = ClientMessage {
            command: ServerCommand::INIT_TICK_SUB,
            payload: Box::into_raw(def) as *mut c_void,
        };

        unsafe { push_client_message(msg, self.server_environment) };

        Ok(Box::new(rx))
    }

    fn send_message(&mut self, code: usize) {
        unimplemented!();
    }
}

fn login(log_env: *mut c_void) -> Result<*mut c_void, String> {
    let username  = CString::new(CONF.fxcm_username).unwrap();
    let password  = CString::new(CONF.fxcm_password).unwrap();
    let url       = CString::new(CONF.fxcm_url).unwrap();

    let res = unsafe { fxcm_login(username.as_ptr(), password.as_ptr(), url.as_ptr(), false, Some(log_cb), log_env) };
    if res.is_null() {
        Err(String::from("The external login function returned NULL; the FXCM servers are likely down."))
    } else {
        Ok(res)
    }
}

/// Tests the ability to log in to FXCM via the C++ code in the library.
#[test]
fn login_test() {
    let username      = CString::new(CONF.fxcm_username).unwrap();
    let mut password  = CString::new(CONF.fxcm_password).unwrap();
    let url           = CString::new(CONF.fxcm_url).unwrap();
    let mut success: bool;
    unsafe {
        success = test_login(username.as_ptr(), password.as_ptr(), url.as_ptr(), false);
    }
    assert!(success, "Error during remote function call; unable to connect to broker.");

    password = CString::new("wrongpassword").unwrap();
    unsafe {
        success = test_login(username.as_ptr(), password.as_ptr(), url.as_ptr(), false);
    }
    assert!(!success, "Test function returned true even for bad credentials.");
}

#[test]
fn broker_server() {
    use std::mem::transmute;

    use std::time::Duration;
    // channel with which to receive responses from the server
    let (tx, rx) = unbounded::<BrokerResult>();
    let tx_ptr = &tx as *const _ as *mut c_void;

    let boxed_cs = Box::new(CommandServer::new(Uuid::new_v4(), "FXCM Native Broker"));
    let env_ptr = Box::into_raw(boxed_cs) as *const _ as *mut c_void;

    let env: *mut c_void = unsafe { init_server_environment(Some(handle_message), tx_ptr, Some(log_cb), env_ptr) };
    let ship  = Spaceship(env);
    let ship2 = ship.clone();

    let handle = thread::spawn(move || {
        // TODO: wait until the connection is ready before starting to process messages
        // let session = login();
        // block on the C++ event loop code and start processing messages
        match login(NULL) {
            Ok(conn) => unsafe { start_server(conn, ship.0) },
            Err(msg) => panic!("{}", msg),
        }
    });

    let message = ClientMessage {
        command: ServerCommand::PING,
        payload: NULL,
    };

    thread::spawn(move || {
        for _ in 0..10 {
            unsafe { push_client_message(message.clone(), ship2.0) };
            thread::sleep(Duration::from_millis(1));
        }
        unsafe { push_client_message(ClientMessage {
            command: transmute(100u32), // invalid enum variant to trigger an error on the C side
            payload: NULL,
        }, ship2.0)}
    });

    for (_, _) in rx.wait().take(11).enumerate() {
        // TODO
    }
}

#[test]
fn tickstream_subbing() {
    use std::mem;

    use futures::Future;
    let mut broker = Box::new(FXCMNative::init(HashMap::new()).wait().unwrap().unwrap());
    let rx = Box::new(broker.sub_ticks(String::from("EUR/USD")).unwrap());
    // leak everything to prevent internals from getting dropped and messing with FFI
    // only necessary since we're not doing anything with the tickstream.
    // TODO: handle this contingency (maybe some kind of reference-counted thing?)
    mem::forget(rx);
    mem::forget(broker);
}

#[test]
fn cstring_conversion() {
    unsafe {
        // allocate a native c-string using system allocator
        let s: &[u8] = b"An example str.\x00";
        let ptr = libc::malloc(16) as *mut c_char;
        libc::strncpy(ptr, s.as_ptr() as *const c_char, s.len());
        let res: CString = ptr_to_cstring(ptr);
        assert_eq!(res.to_str().unwrap(), str::from_utf8(&s[0..15]).unwrap());
        libc::free(ptr as *mut c_void);
    }
}

#[bench]
fn ptr_to_cstring_16(b: &mut test::Bencher) {
    unsafe {
        let s: &[u8] = b"An example str.\x00";
        let ptr = libc::malloc(16) as *mut c_char;
        libc::strncpy(ptr, s.as_ptr() as *const c_char, s.len());
        b.iter(|| {
            let _ = ptr_to_cstring(ptr);
        });
        // free the string because it's the right thing to do
        libc::free(ptr as *mut c_void);
    }
}

#[bench]
fn ptr_to_cstring_64(b: &mut test::Bencher) {
    unsafe {
        let s: &[u8] = b"An example string that continues on much longer than before....\x00";
        let ptr = libc::malloc(64) as *mut c_char;
        libc::strncpy(ptr, s.as_ptr() as *const c_char, s.len());
        b.iter(|| {
            let _ = ptr_to_cstring(ptr);
        });
        // free the string because it's the right thing to do
        libc::free(ptr as *mut c_void);
    }
}

#[bench]
fn ptr_to_cstring_1024(b: &mut test::Bencher) {
    unsafe {
        let mut v = Vec::with_capacity(1024);
        for _ in 0..1024 {
            v.push(69u8);
        }
        let s = v.as_slice();
        let ptr = libc::malloc(1024) as *mut c_char;
        libc::strncpy(ptr, s.as_ptr() as *const c_char, s.len());
        b.iter(|| {
            let _ = ptr_to_cstring(ptr);
        });
        // free the string because it's the right thing to do
        libc::free(ptr as *mut c_void);
    }
}
