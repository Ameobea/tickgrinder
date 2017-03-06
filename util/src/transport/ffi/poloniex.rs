//! FFI methods for communicating with the Poloniex streaming API

use std::collections::HashMap;
use libc::{c_int, c_char, c_void};

use uuid::Uuid;

use trading::tick::GenTick;
use transport::tickstream::maps::poloniex::{PoloniexTradeMap, PoloniexBookModifyMap, PoloniexBookRemovalMap, PolniexOrderBookModification};
use transport::tickstream::sinks::csv_sink::CsvSink;
use transport::tickstream::generics::{GenTickSink, GenTickMap};
use transport::command_server::CommandServer;
use super::*;

pub const POLONIEX_BOOK_MODIFY: c_int = 25;
pub const POLONIEX_BOOK_REMOVE: c_int = 26;
pub const POLONIEX_NEW_TRADE: c_int = 27;
pub const POLONIEX_TROLLBOX: c_int = 28;
pub const POLONIEX_TICKER: c_int = 29;

/// Given a sink ID, returns a `GenTickSink` for it.  I'm putting a lot of trust in you not providing sinks that don't work, because
/// that's going to panic for now.  Also panics if you provide it a bad argument.
unsafe fn get_poloniex_gen_sink_wrapper(sink_id: c_int, arg1: *mut c_void, arg2: *mut c_void) -> Box<GenTickSink<PolniexOrderBookModification>> {
    // TODO: Convert to returning a nullptr/error message kind of thing instead of just dying so that we can make this dynamic
    match sink_id {
        CSV => {
            let path_string = String::from(ptr_to_cstring(arg1 as *mut c_char).to_str().expect("Bad CString provided as argument to CSV Sink!"));
            let mut settings = HashMap::new();
            settings.insert(String::from("output_path"), path_string);
            Box::new(CsvSink::new(settings).expect("Unable to create `CsvSink` from supplied settings!"))
        },
        _ => unimplemented!(),
    }
}

/// Piece of state holding the maps and sinks required to process JSON-encoded order book modifications into a sink
struct BookModificationExecutor {
    map: PoloniexBookModifyMap,
    sink: Box<GenTickSink<PolniexOrderBookModification>>,
    cs: CommandServer,
}

/// Sets up a processing pipeline for converting the JSON messages provided by the Poloniex WebSocket API into `PolniexOrderBookModification`s and
/// then processing those into a sink.  This function returns a pointer to some state that can be passed into the `process_book_modification` function
/// to process that book modification into the sink.
#[no_mangle]
pub unsafe extern "C" fn get_book_modify_executor(sink_id: c_int, arg1: *mut c_void, arg2: *mut c_void) -> *mut c_void {
    let cs = CommandServer::new(Uuid::new_v4(), "Poloniex Order Book Modification Executor");

    Box::into_raw(Box::new(BookModificationExecutor {
        map: PoloniexBookModifyMap::new(HashMap::new(), cs.clone()),
        sink: get_poloniex_gen_sink_wrapper(sink_id, arg1, arg1),
        cs: cs,
    })) as *mut c_void
}

/// Given the state produced by the `get_book_modify_executor` function and an order book modifiction in the form of a c string pointer, processes
/// it into the native representation and after that into the supplied sink.  Timestamp is Unix timestamp is milliseconds.
#[no_mangle]
pub unsafe extern "C" fn process_book_modification(state_ptr: *mut c_void, timestamp: i64, modification: *mut c_char) {
    // convert the supplied pointer into the state
    let state: &mut BookModificationExecutor = &mut *(state_ptr as *mut BookModificationExecutor);
    // convert the supplied c string into a `String` and process it into the map
    let json = String::from(match ptr_to_cstring(modification).to_str() {
        Ok(s) => s,
        Err(e) => {
            state.cs.error(None, &format!("Error while converting cstring to `&str`: {:?}", e));
            return;
        }
    });

    let mapped: Option<GenTick<PolniexOrderBookModification>> = state.map.map(GenTick {
        timestamp: timestamp as u64 * 1000 * 1000, // convert millisecond timestamp into nanos
        data: json,
    });

    // pass the mapped tick into the sink if it exists
    match mapped {
        Some(gt) => state.sink.tick(gt),
        None => (),
    };
}

// TODO: Create master function that takes an ID for the kind of data stream it is and an ID for the sink and does the magic automatically
// that's the only think that will justify this horrible complicated process and the amount of generic extensibility added
