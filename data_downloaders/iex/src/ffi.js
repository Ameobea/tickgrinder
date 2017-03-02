//! Hooks into the tickgrinder utility library compiled from rust to gain access to native platform functionality including data transferring,
//! logging, and tick generator/sink functionality

const ffi = require('ffi');
const ref = require('ref');

const FLATFILE = 0; // { filename: String }
const POSTGRES = 1;// { table: String },
const REDIS_CHANNEL = 2; // { host: String, channel: String },
const REDIS_SET = 3; // { host: String, set_name: String },
const CONSOLE = 4;

const stringPtr = ref.refType(ref.types.CString);

/**
 * Contains the exported API of libtickgrinder
 */
export const TickgrinderUtil = ffi.Library('../../dist/lib/libtickgrinder_util', {
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
  'exec_c_rx_closure': ['void', ['pointer', 'int64', 'int64', 'int64']]
});

/**
 * Returns a wrapper around a `CommandServer` that can be used to send log messages to the platform
 */
export const Log = {
  debug: (category: string, msg: string) => {
    TickgrinderUtil.c_cs_debug(ref.allocCString(category), ref.allocCString(msg));
  },
  notice: TickgrinderUtil.c_cs_notice,
  warning: TickgrinderUtil.c_cs_warning,
  error: TickgrinderUtil.c_cs_error,
  critical: TickgrinderUtil.c_cs_critical,
};

/**
 * Returns a pointer to a native `RxClosure` that is used to push ticks into a sink.
 */
export const getRxClosure = (): ref.types.pointer => {
  // TODO
  return TickgrinderUtil.c_get_rx_closure(REDIS_CHANNEL, ref.allocCString('redis://localhost/'), ref.allocCString('TICKS_TEST'));
};
