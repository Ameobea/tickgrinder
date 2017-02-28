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
  // data transfer function
  // equivalent to `c_transfer_data(src, dst, src_arg1, src_arg2, dst_arg1, dst_arg2, cs)``
  'c_transfer_data': ['bool', ['int', 'int', 'pointer', 'pointer', 'pointer', 'pointer', 'pointer']]
});

const IEX_TOPS_ENDPOINT = 'https://ws-api.iextrading.com/1.0/tops';

const socket = io(IEX_TOPS_ENDPOINT);
const cs = TickgrinderUtil.get_command_server(ref.allocCString('IEX Data Downloader'));
if(cs.isNull()) {
  console.log('Attempt to create a `CommandServer` returned a null pointer.');
  // process.exit(1);
}

const handleMessage = msg => {
  TickgrinderUtil.c_cs_debug(cs, ref.allocCString(''), ref.allocCString(JSON.stringify(msg)));// + JSON.stringify(msg));
};

socket.on('message', handleMessage);

socket.on('connect_failed', function() {
  console.log('Sorry, there seems to be an issue with the connection!');
});

socket.on('connecting', function() {
  console.log('Connecting...');
});

socket.on('connected', function() {
  console.log('Connected.');
});

// socket.emit('subscribe', 'jnug,fb+')
// socket.emit('unsubscribe', 'aig+')

socket.emit('subscribe', 'firehose');

console.log(socket);
