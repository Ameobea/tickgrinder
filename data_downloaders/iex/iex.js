//! The IEX streaming data downloader.  Hooks into the free IEX socket.io API to retrieve live top-of-book information
// @flow

const io = require('socket.io-client');
const ref = require('ref');

import { initWs, handleCommand } from './src/commands';
import util from 'tickgrinder_util';
const { TickgrinderUtil, Log, getRxClosure } = util.ffi;
const CONF = require('./src/conf.js');

// set up environment for the download
const socket = io(CONF.iex_data_downloader_tops_url);
const cs = TickgrinderUtil.get_command_server(ref.allocCString('IEX Data Downloader'));
if(cs.isNull()) {
  console.error('Attempt to create a `CommandServer` returned a null pointer.');
  process.exit(1);
}

// usage: node manager.js uuid
let our_uuid: string = process.argv[2];

if(!our_uuid) {
  console.error('Usage: node manager.js uuid');
  process.exit(0);
} else {
  Log.notice(cs, '', `IEX Data Downloader now listening for commands on ${CONF.redis_control_channel} and ${our_uuid}`);
}

// start listening for commands from the platform and responding to them
initWs(handleCommand);

/**
 * Starts recording live ticks from the IEX exchange and writing them to the supplied endpiont.
 */
function startDownload(symbols: string[]) {
  const rxClosure = getRxClosure(); // TODO

  /**
   * Processing an incoming message from the IEX socket server which contains a new top of book data update
   */
  const handleWsMessage = (msg: {
    symbol: string,
    market_percent: number,
    bidSize: number,
    bidPrice: number,
    askSize: number,
    askPrice: number,
    volume: number,
    lastSalePrice: number,
    lastSaleSize: number,
    lastSaleTime: number,
    lastUpdated: number
  }) => {
    // TickgrinderUtil.c_cs_debug(cs, ref.allocCString(''), ref.allocCString(JSON.stringify(msg)));// + JSON.stringify(msg));
    // console.log(msg);
    TickgrinderUtil.exec_c_rx_closure(rxClosure, msg.lastUpdated, msg.bidPrice * 100, msg.askPrice * 100);
  };

  socket.on('message', handleWsMessage);

  socket.on('connect_failed', function() {
    Log.error(cs, '', 'Unable to connect to the IEX socket.io API server!');
  });

  socket.on('connecting', function() {
    Log.notice(cs, '', 'Connecting to IEX socket.io API server...');
  });

  socket.on('connected', function() {
    Log.notice(cs, '', 'Successfully connected to IEX socket.io API server.');
  });

  socket.emit('subscribe', symbols.join(','));
}
