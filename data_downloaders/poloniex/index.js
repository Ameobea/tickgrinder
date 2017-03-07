//! Data downloader that connects to the Poloniex Websocket API to record live streaming DOM data
// @flow

const https = require('https');
const assert = require('assert');

const autobahn = require('autobahn');

const CONF = require('./src/conf');
import util from 'tickgrinder_util';
const { Tickstream, Log, POLONIEX_BOOK_MODIFY, POLONIEX_BOOK_REMOVE, POLONIEX_NEW_TRADE } = util.ffi;

type OrderBookMessage = {data: {type: string, rate: string, amount: ?string, tradeID: ?string, date: ?string, total: ?string}, type: string};
type CacheEntry = {msg: Array<OrderBookMessage>, seq: number};

// how large to grow the cache before writing the data into the sink.  Must be a multiple of 10.
const CACHE_SIZE = 50;

if(CACHE_SIZE % 10 !== 0) {
  console.error('ERROR: `CACHE_SIZE` must be a multiple of 10!');
  process.exit(1);
}

// TODO: shared node_modules directories for all platform modules that are written in NodeJS
// TODO: Make dynamic and make it a fully fledged spawnable instance
// TODO: Periodic ledger synchronization to make sure we're keeping up correctly

const pair = 'BTC_XMR';
// a place to hold out-of-order messages until the missing messages are received
var messageCache: Array<CacheEntry> = [];
// set up some state for communicating with the platform's util library through FFI
let cs = Log.get_cs('Poloniex Data Downloader');
Log.debug(cs, 'test', 'test');

// create the executors for processing the ticks into the CSV sink
let book_modify_executor = Tickstream.getCsvSinkExecutor(POLONIEX_BOOK_MODIFY, `${CONF.data_dir + '/polo_book_modificiation.csv'}`);
let book_remove_executor = Tickstream.getCsvSinkExecutor(POLONIEX_BOOK_REMOVE, `${CONF.data_dir + '/polo_book_removal.csv'}`);
let book_new_trade_executor = Tickstream.getCsvSinkExecutor(POLONIEX_NEW_TRADE, `${CONF.data_dir + '/polo_book_rew_trade.csv'}`);

/**
 * Attempts to drain the cache of all stored messages
 */
function drainCache() {
  // sort the message cache by sequence number from most recent to oldest
  messageCache = messageCache.sort((a: CacheEntry, b: CacheEntry): number => {
    return (a.seq < b.seq) ? 1 : ((b.seq < a.seq) ? -1 : 0);
  });

  // make sure it's sorted correctly
  assert(messageCache[0].seq > messageCache[messageCache.length - 1].seq);

  // split the oldest 90% of the array off to process into the sink
  let split = messageCache.splice(0, .9 * CACHE_SIZE);
  assert(split.length === .9 * CACHE_SIZE);

  // process the oldest 90% of messages that were waiting for this message before being recorded
  let old_length = split.length;
  for(var j=0; j<old_length; j++) {
    let entry: CacheEntry = split.pop();
    // process each of the individual events in the message
    for(var k=0; k<entry.msg.length; k++) {
      processOrderBookMessage(entry.msg[k]);
    }
  }

  // make sure that the correct number of elements are left in the message cache
  assert(messageCache.length == .1 * CACHE_SIZE);

  if(split.length !== 0) {
    Log.error(cs, 'Message Cache', 'Error while draining message cache: The cache was expected to be empty but had elements remaining in it!');
  }
}

/**
 * Given an in-order message received on the orderbook channel, parses it and submits it to the correct recording endpoint.
 */
function processOrderBookMessage(msg: OrderBookMessage) {
  if(msg.type == 'orderBookModify') {
    recordBookModification(msg.data.rate, msg.data.type, msg.data.amount);
  } else if(msg.type == 'orderBookRemove') {
    recordBookRemoval(msg.data.rate, msg.data.type);
  } else if(msg.type == 'newTrade') {
    recordBookNewTrade(msg.data);
  } else { // TODO: Add handlers for other message types
    Log.error(cs, 'processOrderBookMessage', `Unhandled message type received: ${msg.type}`);
  }
}

// TODO: (IMPORTANT) change the timestamps of data points to be recorded when the point is received instead of when the cache is drained

/**
 * Called for every received order book modification that is in-order.
 */
function recordBookModification(rate: string, type: string, amount: ?string) {
  if(amount == null) {
    amount = '0.0';
    Log.error(cs, 'Message Cache', 'Received a `orderBookModify` message without an `amount` parameter');
  }
  let obj = {rate: rate, type: type, amount: amount};
  let d = new Date();
  // the following lines will remain as a tribute to the monumental effort related to a JavaScript classic "silent fail" of a FFI integer overflow
  // console.log('Writing book modification into tickstream executor...');
  // console.log(book_modify_executor.ref());
  // console.log(JSON.stringify(obj));
  // debugger;
  // push the tick through the processing pipeline to its ultimate destionation.
  Tickstream.executorExec(book_modify_executor, d.getTime(), JSON.stringify(obj));
  // console.log('after executor write');
}

/**
 * Called for every received order book removal that is in-order
 */
function recordBookRemoval(rate: string, type: string) {
  let obj = {rate: rate, type: type};
  let d = new Date();
  Tickstream.executorExec(book_remove_executor, d.getTime(), JSON.stringify(obj));
}

/**
 * Called for every new trade that occurs on the book that is in-order
 */
function recordBookNewTrade(data: {tradeID: ?string, rate: string, amount: ?string, date: ?string, total: ?string, type: string}) {
  let d = new Date();
  Tickstream.executorExec(book_new_trade_executor, d.getTime(), JSON.stringify(data)); // TODO
}

// fetch an image of the order book after giving the recorder a while to fire up
setTimeout(() => {
  https.get(`${CONF.poloniex_http_api_url}?command=returnOrderBook&currencyPair=${pair}&depth=1000000000`, res => {
    res.setEncoding('utf8');
    let rawData = '';

    res.on('data', d => {
      rawData += d;
    }).on('error', e => {
      console.error(`Unable to fetch initial copy of order book: ${e}`);
      process.exit(1);
    });

    res.on('end', () => {
      let last_seq = 0;
      try {
        let parsedData = JSON.parse(rawData);
        last_seq = parsedData.seq;

        // TODO: Read all of the updates in the ledger into the cache as simulated updates
      } catch(e) {
        console.error(`Unable to parse orderbook response into JSON: ${e}`);
        process.exit(1);
      }

      // drop all recorded updates that were before the order book's sequence number
      messageCache = messageCache.filter((msg: CacheEntry): boolean => msg.seq > last_seq);
      console.log(`Received original copy of ledger with seq ${last_seq}; clearing cache.`);
    });
  });
}, 3674);

// creates a new connection to the API endpoint
var connection = new autobahn.Connection({
  url: CONF.poloniex_ws_api_url,
  realm: 'realm1'
});

connection.onopen = session => {
  function marketEvent(args: Array<OrderBookMessage>, kwargs: {seq: number}) {
    messageCache.push({msg: args, seq: kwargs.seq});
    // if the cache is full, sort it and process it into the sink
    if(messageCache.length >= CACHE_SIZE) {
      drainCache();
    }
  }

  function tickerEvent(args, kwargs) {
		// console.logkw(args); // TODO
  }

  function trollboxEvent(args, kwargs) {
		// console.log(args); // TODO
  }

  session.subscribe(pair, marketEvent);
  session.subscribe('ticker', tickerEvent);
  session.subscribe('trollbox', trollboxEvent);
};

connection.onclose = function () {
  Log.critical(cs, 'Websocket', 'Websocket connection closed!')
  console.error('Websocket connection closed!');
};

connection.open();
