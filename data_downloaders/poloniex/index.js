//! Data downloader that connects to the Poloniex Websocket API to record live streaming DOM data
// @flow

const https = require('https');
const assert = require('assert');

const autobahn = require('autobahn');

const CONF = require('./src/conf');
import util from 'tickgrinder_util';
const { Tickstream, Log, POLONIEX_BOOK_MODIFY, POLONIEX_BOOK_REMOVE, POLONIEX_NEW_TRADE } = util.ffi;

type OrderBookMessage = {data: {type: string, rate: string, amount: ?string, tradeID: ?string, date: ?string, total: ?string}, type: string, timestamp: number};
type CacheEntry = {msg: Array<OrderBookMessage>, seq: number, timestamp: number};
type PoloniexRawTrade = {globtalTradeID: number, tradeID: number, date: string, type: string, rate: string, amount: string, total: string};
type RunningDownload = {pair: string, startTime: number, endTime: number, curTime: number};
type ExecutorDescriptor = {id: number, pointer: any};

// how large to grow the cache before writing the data into the sink.  Must be a multiple of 10.
const CACHE_SIZE = 50000;
const SECONDS_IN_A_YEAR = 31556926;

if(CACHE_SIZE % 10 !== 0) {
  console.error('ERROR: `CACHE_SIZE` must be a multiple of 10!');
  process.exit(1);
}

// TODO: Make dynamic and make it a fully fledged spawnable instance
// TODO: Periodic ledger synchronization to make sure we're keeping up correctly

const pair = 'BTC_XMR';
// a place to hold out-of-order messages until the missing messages are received
var messageCache: Array<CacheEntry> = [];
// holds all active downloads and their progress, used to respond to commands requesting their progress
var runningDownloads: Array<RunningDownload> = [];

// set up some state for communicating with the platform's util library through FFI
let cs = Log.get_cs('Poloniex Data Downloader');

// create the executors for processing the ticks into the CSV sink
let book_modify_executor: ExecutorDescriptor =
  Tickstream.getCsvSinkExecutor(POLONIEX_BOOK_MODIFY, `${CONF.data_dir + '/polo_book_modificiation.csv'}`);
let book_remove_executor: ExecutorDescriptor =
  Tickstream.getCsvSinkExecutor(POLONIEX_BOOK_REMOVE, `${CONF.data_dir + '/polo_book_removal.csv'}`);
let book_new_trade_executor: ExecutorDescriptor =
  Tickstream.getCsvSinkExecutor(POLONIEX_NEW_TRADE, `${CONF.data_dir + '/polo_book_rew_trade.csv'}`);

// usage: ./run.sh uuid
let our_uuid: string = process.argv[2];

if(!our_uuid) {
  console.error('Usage: node manager.js uuid');
  process.exit(0);
} else {
  Log.notice(cs, '', `Poloniex Data Downloader now listening for commands on ${CONF.redis_control_channel} and ${our_uuid}`);
}

function handleWsMessage(dispatch: any, msg: any) {
  // TODO
}

// set up Websocket connection to the platform's messaging system
let socket = util.initWs(handleWsMessage, null, our_uuid, wsError);

// send ready message to notify the platform that we're up and running
let wsmsg = {uuid: v4(), channel: CONF.redis_control_channel, message: JSON.stringify("Ready")};
socket.send(JSON.stringify(wsmsg));

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
      processOrderBookMessage(entry.msg[k], entry.timestamp);
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
function processOrderBookMessage(msg: OrderBookMessage, timestamp: number) {
  if(msg.type == 'orderBookModify') {
    recordBookModification(msg.data.rate, msg.data.type, msg.data.amount, timestamp);
  } else if(msg.type == 'orderBookRemove') {
    recordBookRemoval(msg.data.rate, msg.data.type, timestamp);
  } else if(msg.type == 'newTrade') {
    recordBookNewTrade(msg.data, timestamp);
  } else { // TODO: Add handlers for other message types
    Log.error(cs, 'processOrderBookMessage', `Unhandled message type received: ${msg.type}`);
  }
}

/**
 * Called for every received order book modification that is in-order.
 */
function recordBookModification(rate: string, type: string, amount: ?string, timestamp: number) {
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
  Tickstream.executorExec(book_modify_executor, timestamp, JSON.stringify(obj));
  // console.log('after executor write');
}

/**
 * Called for every received order book removal that is in-order
 */
function recordBookRemoval(rate: string, type: string, timestamp: number) {
  let obj = {rate: rate, type: type};
  Tickstream.executorExec(book_remove_executor, timestamp, JSON.stringify(obj));
}

/**
 * Called for every new trade that occurs on the book that is in-order
 */
function recordBookNewTrade(data: {tradeID: ?string, rate: string, amount: ?string, date: ?string, total: ?string, type: string}, timestamp: number) {
  Tickstream.executorExec(book_new_trade_executor, timestamp, JSON.stringify(data)); // TODO
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
      Log.notice(cs, 'Ledger Downloader', `Received original copy of ledger with seq ${last_seq}; clearing cache.`);
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

connection.onclose = function() {
  Log.warning(cs, 'Websocket', 'Websocket connection closed!');
  console.error('Websocket connection closed!');
};

connection.open();

/**
 * Queries the public API to return the last [count] trades for a given pair.  Results are limited to 50,000 trades and the supplied
 * window must be less that one year in size.  Trades are returned in reverse chronological order.  If there are over 50,000 trades in
 * the supplied result, the oldest trades will be truncated.
 * @param {string} pair - The pair of trade history to download formatted like "BTC_XMR"
 * @param {number} startTimestamp - The Unix timestamp of the start of the download window, second precision.
 * @param {number} endTimestamp - The Unix timestamp of the end of the download window, second precision.
 */
function fetchTradeHistory(pair: string, startTimestamp: number, endTimestamp: number)
  :Promise<Array<PoloniexRawTrade>>
{
  return new Promise((fulfill, reject) => {
    https.get(`${CONF.poloniex_http_api_url}?command=returnTradeHistory&currencyPair=${pair}&start=${startTimestamp}&end=${endTimestamp}`, res => {
      res.setEncoding('utf8');
      let rawData = '';

      res.on('data', d => {
        rawData += d;
      }).on('error', e => {
        Log.error('', `Unable to feth historical trade data for pair ${pair}: ${e}`);
        reject(e);
      });

      res.on('end', () => {
        try {
          let parsed: Array<PoloniexRawTrade> = JSON.parse(rawData);
          fulfill(parsed);
        } catch(e) {
          Log.error('', `Error while parsing JSON response from Poloniex API during historical trade fetching: ${e}`);
          reject(e);
        }
      });
    });
  });
}

/**
 * Initializes a download of historical data for a pair.  This will attempt to fetch all of the stored data over that time period and
 * write it to the sink.  This will internally manage the API limitations.  Start/end timestamps are Unix format second precision.
 */
function initHistTradeDownload(pair: string, startTimestamp: number, endTimestamp: number, outputPath: string) {
  // create an executor to which to funnel the data
  let executor: ExecutorDescriptor = Tickstream.getCsvSinkExecutor(POLONIEX_NEW_TRADE, outputPath);
  // register this download as in-progress
  runningDownloads.push({pair: pair, startTime: startTimestamp, endTime: endTimestamp, curTime: startTimestamp});
  // send download started message
  let wsmsg = {uuid: uuid, channel: channel, message: JSON.stringify(msg)};

  let curStartTimestamp = startTimestamp;
  let curEndTimestamp = endTimestamp;
  // make sure that we're requesting less than a year's worth of trades to start off
  if(endTimestamp - curStartTimestamp > SECONDS_IN_A_YEAR) {
    curEndTimestamp = curStartTimestamp + (SECONDS_IN_A_YEAR - 1001);
  } else {
    curEndTimestamp = endTimestamp;
  }

  // TODO: Make the loop properly and probably add some kind of dowload delay
  fetchTradeHistory(pair, curStartTimestamp, curEndTimestamp).then((data: Array<PoloniexRawTrade>) => {
    // sort the trades from oldest to newest
    let sortedData = data.sort((a: PoloniexRawTrade, b: PoloniexRawTrade): number => {
      return (a.tradeID > b.tradeID) ? 1 : ((b.tradeID > a.tradeID) ? -1 : 0);
    });
    assert(sortedData[sortedData.length - 1].tradeID > sortedData[0].tradeID);

    // process the trades into the sink
    for(var i=0; i<sortedData.length; i++) {
      // convert the date from "2017-03-10 01:31:08" format into a Unix timestamp
      let timestamp = new Date(sortedData[i].date);
      // send the trade through the executor and into the sink
      Tickstream.executorExec(executor, timestamp, JSON.stringify(sortedData[i]));
    }

    // update download progress
    // TODO
    // TODO: Make the download progress thing a key:value store rather than an array
    // TODO: Make platform respond to messages including download requests, download progresses, etc.

    if(sortedData.length === 50000) {
      // if it was more than 50,000 trades, download what's missing before going on
      curEndTimestamp = new Date(sortedData[0].date).getTime() - 1;
    } else {
      curStartTimestamp = curEndTimestamp + 1;
      // if less than 50,000 trades, then download the next segment
      if(endTimestamp - curEndTimestamp > SECONDS_IN_A_YEAR) {
        curEndTimestamp = curStartTimestamp + (SECONDS_IN_A_YEAR - 1001);
      } else if(curEndTimestamp >= endTimestamp) {
        // download complete!
        break;
      } else {
        curEndTimestamp = endTimestamp;
      }
    }
  });

  // send download complete message
  // TODO
}
