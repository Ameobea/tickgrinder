//! Hooks into the Poloniex WebSocket API to retrieve streams of live data, feeds them through executors, and into sinks.
// @flow

const assert = require('assert');
const https = require('https');

const autobahn = require('autobahn');

const CONF = require('./conf');
import util from 'tickgrinder_util';
const { Tickstream, Log, POLONIEX_BOOK_MODIFY, POLONIEX_BOOK_REMOVE, POLONIEX_NEW_TRADE } = util.ffi;

type OrderBookMessage = {data: {type: string, rate: string, amount: ?string, tradeID: ?string, date: ?string, total: ?string}, type: string, timestamp: number};
type CacheEntry = {msg: Array<OrderBookMessage>, seq: number, timestamp: number};
type ExecutorDescriptor = {id: number, pointer: any};

// how large to grow the cache before writing the data into the sink.  Must be a multiple of 10.
const CACHE_SIZE = 5000;

if(CACHE_SIZE % 10 !== 0) {
  console.error('ERROR: `CACHE_SIZE` must be a multiple of 10!');
  process.exit(1);
}

/**
 * Starts downloading live streaming trade and orderbook data to the specified destination.
 * TODO: Periodic ledger synchronization to make sure we're keeping up correctly
 * @param {function} isDownloadRunning - A function that returns whether or not the selected download has been cancelled
 */
function startWsDownload(pair: string, dst: any, cs: any, isDownloadCancelled: () => boolean): {Error: {status: string}} | string {
  // create the executors used for processing the ticks
  let book_modify_executor: ExecutorDescriptor =
    Tickstream.getCsvSinkExecutor(POLONIEX_BOOK_MODIFY, dst.Flatfile.filename + '.modifications.csv');
  let book_remove_executor: ExecutorDescriptor =
    Tickstream.getCsvSinkExecutor(POLONIEX_BOOK_REMOVE, dst.Flatfile.filename + '.removals.csv');
  let book_new_trade_executor: ExecutorDescriptor =
    Tickstream.getCsvSinkExecutor(POLONIEX_NEW_TRADE, dst.Flatfile.filename + '.newtrades.csv');

  // create a cache to store messages that are waiting to be pushed through the executor
  let messageCache: Array<CacheEntry> = [];

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
        let lastSeq = 0;
        try {
          let parsedData = JSON.parse(rawData);
          lastSeq = parsedData.seq;
        } catch(e) {
          console.error(`Unable to parse orderbook response into JSON: ${e}`);
          process.exit(1);
        }

        // drop all recorded updates that were before the order book's sequence number
        messageCache = messageCache.filter((msg: CacheEntry): boolean => msg.seq > lastSeq);
        Log.notice(cs, 'Ledger Downloader', `Received original copy of ledger with seq ${lastSeq}; clearing cache.`);
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
      messageCache.push({msg: args, seq: kwargs.seq, timestamp: Date.now()});
      if(isDownloadCancelled()) {
        // flush all cached ticks into the sink before exiting
        drainCache(true, messageCache, book_modify_executor, book_remove_executor, book_new_trade_executor, cs);
        // close the connection so that no further updates are recorded
        connection.close();
        Log.info(cs, '', `Data download for symbol ${pair} cancelled.`)
        return;
      }
      // if the cache is full, sort it and process it into the sink
      if(messageCache.length >= CACHE_SIZE) {
        drainCache(false, messageCache, book_modify_executor, book_remove_executor, book_new_trade_executor, cs);
      }
    }

    session.subscribe(pair, marketEvent);
  };

  connection.onclose = function() {
    Log.warning(cs, 'Socket.io', 'Socket.io connection closed!');
    console.error('Socket.io connection closed!');
  };

  connection.open();

  return 'Ok';
}

/**
 * Attempts to drain the cache of all stored messages
 * @param {bool} isFlush - If true, the entire cache will be emptied into the sink.  If false, only the oldest 90% will be emptied in order
 *  allow for any out-of-order ticks to be properly ordered.
 */
function drainCache(
  isFlush: boolean, messageCache: Array<CacheEntry>, book_modify_executor: ExecutorDescriptor, book_remove_executor: ExecutorDescriptor,
  book_new_trade_executor: ExecutorDescriptor, cs
) {
  // sort the message cache by sequence number from most recent to oldest
  messageCache = messageCache.sort((a: CacheEntry, b: CacheEntry): number => {
    return (a.seq < b.seq) ? 1 : ((b.seq < a.seq) ? -1 : 0);
  });

  // make sure it's sorted correctly
  assert(messageCache[0].seq > messageCache[messageCache.length - 1].seq);

  // split the oldest 90% of the array off to process into the sink
  let split;
  if(!isFlush) {
    split = messageCache.splice(0, .9 * CACHE_SIZE);
    assert(split.length === .9 * CACHE_SIZE);
    // make sure that the correct number of elements are left in the message cache
    assert(messageCache.length == .1 * CACHE_SIZE);
  } else {
    split = messageCache;
    messageCache = [];
  }

  // process the messages that were waiting for this message before being recorded
  let old_length = split.length;
  for(var j=0; j<old_length; j++) {
    let entry: CacheEntry = split.pop();
    // process each of the individual events in the message
    for(var k=0; k<entry.msg.length; k++) {
      processOrderBookMessage(entry.msg[k], entry.timestamp, book_modify_executor, book_remove_executor, book_new_trade_executor, cs);
    }
  }

  if(split.length !== 0) {
    Log.error(cs, 'Message Cache', 'Error while draining message cache: The cache was expected to be empty but had elements remaining in it!');
  }
}

/**
 * Given an in-order message received on the orderbook channel, parses it and submits it to the correct recording endpoint.
 */
function processOrderBookMessage(
  msg: OrderBookMessage, timestamp: number, book_modify_executor: ExecutorDescriptor,
  book_remove_executor: ExecutorDescriptor, book_new_trade_executor: ExecutorDescriptor, cs: any
) {
  if(msg.type == 'orderBookModify') {
    recordBookModification(msg.data.rate, msg.data.type, msg.data.amount, timestamp, book_modify_executor, cs);
  } else if(msg.type == 'orderBookRemove') {
    Tickstream.executorExec(book_remove_executor, timestamp, JSON.stringify({rate: msg.data.rate, type: msg.data.type}));
  } else if(msg.type == 'newTrade') {
    Tickstream.executorExec(book_new_trade_executor, timestamp, JSON.stringify(msg.data));
  } else {
    Log.error(cs, 'processOrderBookMessage', `Unhandled message type received: ${msg.type}`);
  }
}

/**
 * Called for every received order book modification that is in-order.
 */
function recordBookModification(rate: string, type: string, amount: ?string, timestamp: number, executor: ExecutorDescriptor, cs: any) {
  if(amount == null) {
    amount = '0.0';
    Log.error(cs, 'Message Cache', 'Received a `orderBookModify` message without an `amount` parameter');
  }
  let obj = {rate: rate, type: type, amount: amount};
  // the following lines will remain as a tribute to the monumental effort related to a JavaScript classic "silent fail" of a FFI integer overflow
  // console.log('Writing book modification into tickstream executor...');
  // console.log(book_modify_executor.ref());
  // console.log(JSON.stringify(obj));
  // debugger;
  // push the tick through the processing pipeline to its ultimate destionation.
  Tickstream.executorExec(executor, timestamp, JSON.stringify(obj));
  // console.log('after executor write');
}

module.exports = startWsDownload;
