const io = require('socket.io-client');
const ffi = require('ffi');
const ref = require('ref');

const stringPtr = ref.refType(ref.types.CString);

const TickgrinderUtil = ffi.Library('../../dist/lib/libtickgrinder_util', {
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

const Log = {
  debug: function(category, msg) {
    TickgrinderUtil.c_cs_debug(ref.allocCString(category), ref.allocCString(msg));
  },
  notice: TickgrinderUtil.c_cs_notice,
  warning: TickgrinderUtil.c_cs_warning,
  error: TickgrinderUtil.c_cs_error,
  critical: TickgrinderUtil.c_cs_critical,
};

const IEX_TOPS_ENDPOINT = 'https://ws-api.iextrading.com/1.0/tops';
const FLATFILE = 0; // { filename: String }
const POSTGRES = 1;// { table: String },
const REDIS_CHANNEL = 2; // { host: String, channel: String },
const REDIS_SET = 3; // { host: String, set_name: String },
const CONSOLE = 4;

// set up environment for the download
const socket = io(IEX_TOPS_ENDPOINT);
const cs = TickgrinderUtil.get_command_server(ref.allocCString('IEX Data Downloader'));
if(cs.isNull()) {
  console.log('Attempt to create a `CommandServer` returned a null pointer.');
  // process.exit(1);
}
// TODO: Make dynamic
const rxClosure = TickgrinderUtil.c_get_rx_closure(REDIS_CHANNEL, ref.allocCString('redis://localhost/'), ref.allocCString('TICKS_TEST'));

const handleMessage = msg => {
  // TickgrinderUtil.c_cs_debug(cs, ref.allocCString(''), ref.allocCString(JSON.stringify(msg)));// + JSON.stringify(msg));
  // console.log(msg);
  TickgrinderUtil.exec_c_rx_closure(rxClosure, msg.lastUpdated, msg.bidPrice * 100, msg.askPrice * 100);
};

socket.on('message', handleMessage);

socket.on('connect_failed', function() {
  Log.error('There sseems to be an issue with the connection to the IEX socket.io API server!');
});

socket.on('connecting', function() {
  Log.notice('', 'Connecting to IEX socket.io API server...');
});

socket.on('connected', function() {
  Log.notice('', 'Successfully connected to IEX socket.io API server.');
});

socket.emit('subscribe', 'firehose');
