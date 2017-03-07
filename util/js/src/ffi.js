//! Hooks into the tickgrinder utility library compiled from rust to gain access to native platform functionality including data transferring,
//! logging, and tick generator/sink functionality
// @flow

const ffi = require('ffi');
const ref = require('ref');

const FLATFILE = 0; // { filename: String }
const POSTGRES = 1; // { table: String },
const REDIS_CHANNEL = 2; // { host: String, channel: String },
const REDIS_SET = 3; // { host: String, set_name: String },
const CONSOLE = 4;
const CSV = 5;

const POLONIEX_BOOK_MODIFY = 25;
const POLONIEX_BOOK_REMOVE = 26;
const POLONIEX_NEW_TRADE = 27;
const POLONIEX_TROLLBOX = 28;
const POLONIEX_TICKER = 29;

const stringPtr = ref.refType(ref.types.CString);

/**
 * Contains the exported API of libtickgrinder
 */
const TickgrinderUtil = ffi.Library('libtickgrinder_util', {
  // command server functions
  'get_command_server': ['pointer', [stringPtr]], // Returns a reference to a `CommandServer` given a name for it
  'c_cs_debug': ['void', ['pointer', stringPtr, stringPtr]], // equivalent to `cs.debug(category, msg)`
  'c_cs_notice': ['void', ['pointer', stringPtr, stringPtr]], // equivalent to `cs.notice(category, msg)`
  'c_cs_warning': ['void', ['pointer', stringPtr, stringPtr]], // equivalent to `cs.warning(category, msg)`
  'c_cs_error': ['void', ['pointer', stringPtr, stringPtr]], // equivalent to `cs.error(category, msg)`
  'c_cs_critical': ['void', ['pointer', stringPtr, stringPtr]], // equivalent to `cs.critical(category, msg)`
  // data management functions
  // equivalent to `c_transfer_data(src, dst, src_arg1, src_arg2, dst_arg1, dst_arg2, cs)``
  'c_transfer_data': ['bool', ['int64', 'int64', 'pointer', 'pointer', 'pointer', 'pointer', 'pointer']],
  'c_get_rx_closure': ['pointer', ['int64', 'pointer', 'pointer']],
  'exec_c_rx_closure': ['void', ['pointer', 'int64', 'int64', 'int64']],
  // poloniex data functions
  // get_executor(executor_id: i64, sink_id: i64, sink_arg1: *mut c_void, sink_arg2: *mut c_void) -> *mut c_void
  'get_executor': ['pointer', ['int64', 'int64', 'pointer', 'pointer']],
  // process_event(event_id: i64, state_ptr: *mut c_void, timestamp: i64, event: *mut c_char)
  'process_event': ['void', ['int64', 'pointer', 'int64', 'pointer']]
});

/**
 * Contains functions for creating and working with the data processing pipelines that take the raw JSON-encoded strings provided
 * by the Poloniex WebSocket API and process it down into a sink.
 */
const Tickstream = {
  /**
   * Returns a pointer to a tickstream executor that processes data of the specified type into a CSV sink.  The returned pointer should
   * be used along with `Tickstream.executorExec` to process data into the sink.  `data_type` should be one of the `POLONIEX_TROLLBOX`,
   * `POLONIEX_BOOK_MODIFY`, etc.
   */
  getCsvSinkExecutor: (data_type: number, output_path: string): ref.types.pointer => {
    let pointer = TickgrinderUtil.get_executor(data_type, CSV, ref.allocCString(output_path), null);
    if(pointer.isNull()) {
      console.error('The returned executor pointer was null!');
      process.exit(1);
    }

    return {pointer: pointer, id: data_type};
  },

  /**
   * Given a reference to a tickstream executor provided by `getCsvSinkExecutor` and a JSON-encoded message from its corresponding
   * data source, processes the data into the stream, through the internal map, and into the CSV file sink.  `timestamp` is the
   * milliseconds since the epoch Unix timestamp of the data point.
   */
  executorExec: function(executor: {id: number, pointer: ref.types.pointer}, timestamp: number, json_string: string) {
    TickgrinderUtil.process_event(executor.id, executor.pointer, timestamp, ref.allocCString(json_string));
  }
};

module.exports = {
  FLATFILE: FLATFILE,
  POSTGRES: POSTGRES,
  REDIS_CHANNEL: REDIS_CHANNEL,
  REDIS_SET: REDIS_SET,
  CONSOLE: CONSOLE,
  CSV: CSV,

  POLONIEX_BOOK_MODIFY: POLONIEX_BOOK_MODIFY,
  POLONIEX_BOOK_REMOVE: POLONIEX_BOOK_REMOVE,
  POLONIEX_NEW_TRADE: POLONIEX_NEW_TRADE,
  POLONIEX_TROLLBOX: POLONIEX_TROLLBOX,
  POLONIEX_TICKER: POLONIEX_TICKER,

  TickgrinderUtil: TickgrinderUtil,

  Tickstream: Tickstream,

  /**
   * Returns a wrapper around a `CommandServer` that can be used to send log messages to the platform
   */
  Log: {
    debug: (cs: ref.types.pointer, category: string, msg: string) => {
      TickgrinderUtil.c_cs_debug(cs, ref.allocCString(category), ref.allocCString(msg));
    },
    notice: (cs: ref.types.pointer, category: string, msg: string) => {
      TickgrinderUtil.c_cs_notice(cs, ref.allocCString(category), ref.allocCString(msg));
    },
    warning: (cs: ref.types.pointer, category: string, msg: string) => {
      TickgrinderUtil.c_cs_warning(cs, ref.allocCString(category), ref.allocCString(msg));
    },
    error: (cs: ref.types.pointer, category: string, msg: string) => {
      TickgrinderUtil.c_cs_error(cs, ref.allocCString(category), ref.allocCString(msg));
    },
    critical: (cs: ref.types.pointer, category: string, msg: string) => {
      TickgrinderUtil.c_cs_critical(cs, ref.allocCString(category), ref.allocCString(msg));
    },
    get_cs: (name: string): ref.types.pointer => {
      return TickgrinderUtil.get_command_server(ref.allocCString(name));
    }
  },

  /**
   * Returns a pointer to a native `RxClosure` that is used to push ticks into a sink.
   */
  getRxClosure: (): ref.types.pointer => {
    // TODO
    return TickgrinderUtil.c_get_rx_closure(REDIS_CHANNEL, ref.allocCString('redis://localhost/'), ref.allocCString('TICKS_TEST'));
  },
};
