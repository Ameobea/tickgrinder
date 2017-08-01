//! FFI methods for communicating with the Poloniex streaming API

use std::collections::HashMap;
use libc::{c_int, c_char, c_void};

use serde::{Serialize, Deserialize};
use uuid::Uuid;

use trading::tick::GenTick;
use transport::tickstream::maps::poloniex::{PoloniexBookModifyMap, PoloniexBookRemovalMap, PolniexOrderBookModification};
use transport::tickstream::maps::poloniex::{PoloniexOrderBookRemoval, PoloniexTradeMap, PoloniexTrade};
use transport::tickstream::sinks::csv_sink::CsvSink;
use transport::tickstream::generics::{GenTickSink, GenTickMap};
use transport::command_server::CommandServer;
use super::*;

pub const POLONIEX_BOOK_MODIFY: c_int = 25;
pub const POLONIEX_BOOK_REMOVE: c_int = 26;
pub const POLONIEX_NEW_TRADE: c_int = 27;
pub const POLONIEX_TROLLBOX: c_int = 28;
pub const POLONIEX_TICKER: c_int = 29;

/// An executor contains some state that's used to process raw data, map it into a different format, and send that through a sink.
trait PoloniexExecutor<T> {
    fn get_cs(&mut self) -> &mut CommandServer;

    unsafe fn get_map(&mut self) -> &mut GenTickMap<String, T>;

    fn get_sink(&mut self) -> &mut GenTickSink<T>;

    unsafe fn process_event(&mut self, timestamp: i64, event_json: *mut c_char) {
        // convert the supplied c string into a `String`
        let json = String::from(match ptr_to_cstring(event_json).to_str() {
            Ok(s) => s,
            Err(e) => {
                self.get_cs().error(None, &format!("Error while convering cstring to &str: {:?}", e));
                return;
            }
        });

        // pass the JSON string through the map to convert it to the native struct representation
        let mapped: Option<GenTick<T>> = self.get_map().map(GenTick {
            timestamp: timestamp as u64 * 1000 * 1000, // convert millisecond timestamp into nanos
            data: json,
        });

        // pass the mapped tick into the sink if it was successfully processed through the map without error
        match mapped {
            Some(gt) => self.get_sink().tick(gt),
            None => (),
        };
    }
}

/// Given the ID of the type of executor that's needed, returns a pointer to one.  Internally, this sets up the processing pipeline for
/// taking ticks in, mapping them to the necessary format, and pushing them into the sink.
#[no_mangle]
pub unsafe extern "C" fn get_executor(
    executor_id: i64, sink_id: i64, sink_arg1: *mut c_void, sink_arg2: *mut c_void
) -> *mut c_void {
    // get a properly named `CommandServer` and the correct map (boxed and cast to a *mut c_void) that correspond to the executor
    match executor_id as i32 {
        POLONIEX_BOOK_MODIFY => {
            let cs = CommandServer::new(Uuid::new_v4(), "Poloniex Order Book Modification Executor");
            Box::into_raw(Box::new(BookModificationExecutor {
                map: Box::into_raw(Box::new(PoloniexBookModifyMap::new(HashMap::new(), cs.clone()))) as *mut c_void,
                sink: get_poloniex_gen_sink_wrapper(sink_id as i32, sink_arg1, sink_arg2),
                cs: cs,
            })) as *mut c_void
        },
        POLONIEX_BOOK_REMOVE => {
            let cs = CommandServer::new(Uuid::new_v4(), "Poloniex Order Book Removal Executor");
            Box::into_raw(Box::new(BookRemovalExecutor {
                map: Box::into_raw(Box::new(PoloniexBookRemovalMap::new(HashMap::new(), cs.clone()))) as *mut c_void,
                sink: get_poloniex_gen_sink_wrapper(sink_id as i32, sink_arg1, sink_arg2),
                cs: cs,
            })) as *mut c_void
        },
        POLONIEX_NEW_TRADE => {
            let cs = CommandServer::new(Uuid::new_v4(), "Poloniex New Trade Executor");
            Box::into_raw(Box::new(TradeExecutor {
                map: Box::into_raw(Box::new(PoloniexTradeMap::new(HashMap::new(), cs.clone()))) as *mut c_void,
                sink: get_poloniex_gen_sink_wrapper(sink_id as i32, sink_arg1, sink_arg2),
                cs: cs,
            })) as *mut c_void
        }
        _ => unimplemented!(),
    }
}

/// Function exported to the FFI interface that takes an event type, timestamp, and event as a JSON-formatted string pointer and
/// processes it through the pipeline and into the sink.
#[no_mangle]
pub unsafe extern "C" fn process_event(event_id: i64, state_ptr: *mut c_void, timestamp: i64, event: *mut c_char) {
    match event_id as i32 {
        POLONIEX_BOOK_MODIFY => {
            let executor = &mut *(state_ptr as *mut BookModificationExecutor);
            executor.process_event(timestamp, event);
        },
        POLONIEX_BOOK_REMOVE => {
            let executor = &mut *(state_ptr as *mut BookRemovalExecutor);
            executor.process_event(timestamp, event);
        },
        POLONIEX_NEW_TRADE => {
            let executor = &mut *(state_ptr as *mut TradeExecutor);
            executor.process_event(timestamp, event);
        },
        _ => unimplemented!(),
    };
}

/// Given a sink ID, returns a `GenTickSink` for it.  I'm putting a lot of trust in you not providing sinks that don't work, because
/// that's going to panic for now.  Also panics if you provide it a bad argument.
unsafe fn get_poloniex_gen_sink_wrapper<T>(
    sink_id: c_int, arg1: *mut c_void, arg2: *mut c_void
) -> Box<GenTickSink<T>> where T : Serialize, T : for<'de> Deserialize<'de>, T: 'static {
    // TODO: Convert to returning a nullptr/error message kind of thing instead of just dying so that we can make this dynamic
    match sink_id {
        CSV => {
            let path_string = String::from(ptr_to_cstring(arg1 as *mut c_char).to_str().expect("Bad CString provided as argument to CSV Sink!"));
            let mut settings = HashMap::new();
            settings.insert(String::from("output_path"), path_string);
            Box::new(CsvSink::new(settings).expect("Unable to create `CsvSink` from supplied settings!") as CsvSink<T>)
        },
        _ => unimplemented!(),
    }
}

/// Piece of state holding the maps and sinks required to process JSON-encoded order book modifications into a sink
struct BookModificationExecutor {
    map: *mut c_void,
    sink: Box<GenTickSink<PolniexOrderBookModification>>,
    cs: CommandServer,
}

impl PoloniexExecutor<PolniexOrderBookModification> for BookModificationExecutor {
    fn get_cs(&mut self) -> &mut CommandServer {
        &mut self.cs
    }

    unsafe fn get_map(&mut self) -> &mut GenTickMap<String, PolniexOrderBookModification> {
        &mut *(self.map as *mut PoloniexBookModifyMap)
    }

    fn get_sink(&mut self) -> &mut GenTickSink<PolniexOrderBookModification> {
        &mut *self.sink
    }
}

/// Piece of state holding the maps and sinks required to process JSON-encoded order book removals into a sink
struct BookRemovalExecutor {
    map: *mut c_void,
    sink: Box<GenTickSink<PoloniexOrderBookRemoval>>,
    cs: CommandServer,
}

impl PoloniexExecutor<PoloniexOrderBookRemoval> for BookRemovalExecutor {
    fn get_cs(&mut self) -> &mut CommandServer {
        &mut self.cs
    }

    unsafe fn get_map(&mut self) -> &mut GenTickMap<String, PoloniexOrderBookRemoval> {
        &mut *(self.map as *mut PoloniexBookRemovalMap)
    }

    fn get_sink(&mut self) -> &mut GenTickSink<PoloniexOrderBookRemoval> {
        &mut *self.sink
    }
}

/// Piece of state holding the maps and sinks required to process JSON-encoded trades into a sink
struct TradeExecutor {
    map: *mut c_void,
    sink: Box<GenTickSink<PoloniexTrade>>,
    cs: CommandServer,
}

impl PoloniexExecutor<PoloniexTrade> for TradeExecutor {
    fn get_cs(&mut self) -> &mut CommandServer {
        &mut self.cs
    }

    unsafe fn get_map(&mut self) -> &mut GenTickMap<String, PoloniexTrade> {
        &mut *(self.map as *mut PoloniexTradeMap)
    }

    fn get_sink(&mut self) -> &mut GenTickSink<PoloniexTrade> {
        &mut *self.sink
    }
}
