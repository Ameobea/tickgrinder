//! Data downloader that connects to the Poloniex Websocket API to record live streaming DOM data
// @flow

const https = require('https');

const autobahn = require('autobahn');

const CONF = require('./src/conf');
import util from 'tickgrinder_util';
const { TickgrinderUtil, Log, getRxClosure } = util.ffi;

type OrderBookMessage = {data: {type: string, rate: string, amount: ?string}, type: string};
type CacheEntry = {msg: Array<OrderBookMessage>, seq: number};

// TODO: shared node_modules directories for all platform modules that are writte in NodeJS

/**
 * Attempts to drain the cache of all queued messages
 */
function drainCache() {
  // sort the message cache by sequence number from most recent to oldest
  messageCache = messageCache.sort((a: CacheEntry, b: CacheEntry): number => {
    return (a.seq < b.seq) ? 1 : ((b.seq < a.seq) ? -1 : 0);
  });

  // console.log(messageCache);

  // process any cached messages that were waiting for this message before being recorded
  for(var j=messageCache.length; j--;) {
    if(last_seq + 1 === messageCache[j].seq) {
      let msg = messageCache.pop();
      // process each of the individual events in the message
      for(var k=0; k<msg.msg.length; k++) {
        processOrderBookMessage(msg.msg[k]);
      }
    } else {
      // there's another missing message, so stop processing and begin waiting for the missing one before going on.
      console.log(`Encountered gap while draining cache; Expected ${last_seq + 1} and found ${messageCache[j].seq}`);
      break;
    }

    last_seq += 1;
  }
}

/**
 * Given an in-order message received on the orderbook channel, parses it and submits it to the correct recording endpoint.
 */
function processOrderBookMessage(msg: OrderBookMessage) {
  switch(msg.type) {
  case 'orderBookModify': {
    recordBookModification(parseFloat(msg.data.rate), msg.type == 'bid', parseFloat(msg.data.amount));
    break;
  }
    // TODO: Add handlers for other message types
  default: {
      // TODO: Log error that we received unexpected message type
  }

  }
}

/**
 * Called for every received order book modification that is in-order.
 */
function recordBookModification(rate: number, is_bid: boolean, amount: number) {
  // TODO
  console.log(rate);
}

let cs = Log.get_cs('Poloniex Data Downloader');

// TODO: Make dynamic and make it a fully fledged spawnable instance
const pair = 'BTC_XMR';
// a place to hold out-of-order messages until the missing messages are received
var messageCache: Array<CacheEntry> = [];
// the sequence number of the last received message
var last_seq: number = 0;

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
      try {
        let parsedData = JSON.parse(rawData);
        last_seq = parsedData.seq;
      } catch(e) {
        console.error(`Unable to parse orderbook response into JSON: ${e}`);
        process.exit(1);
      }

      // TODO: Read all of the updates in the ledger into the cache as simulated updates
      // TODO: /TEAR OUT THIS HORRIBLE SYSTEM AND REPLACE IT WITH SOMETHING TINY AND SLIGHTLY LESS HORRIBLE/

      // drop all recorded updates that were before the order book's sequence number
      messageCache = messageCache.filter((msg: CacheEntry): boolean => msg.seq > last_seq);
      // and process all the ones after it into the sink
      console.log(`Received original copy of ledger with seq ${last_seq}; draining cache.`);
      drainCache();
    });
  });
}, 3674)

// creates a new connection to the API endpoint
var connection = new autobahn.Connection({
  url: CONF.poloniex_ws_api_url,
  realm: 'realm1'
});

connection.onopen = session => {
  function marketEvent(args: Array<OrderBookMessage>, kwargs: {seq: number}) {
    if(last_seq !== kwargs.seq - 1) {
      // if there is a gap between the last received sequence number, write to cache instead of recording
      messageCache.push({msg: args, seq: kwargs.seq});
      console.log(`Gap so writing to cache: ${kwargs.seq}`);

      // if the cache is a certain length, just give up on the missing message.
      if(messageCache.length > 50 && last_seq !== 0) {
        console.log('Dropping missing message due to extreme delay.');
        last_seq += 1;

        drainCache();
      }
    } else {
      for(var i=0; i<args.length; i++) {
        // the message is in-order, so process it into the correct endpoint
        processOrderBookMessage(args[i]);
      }

      last_seq += 1;
      // if there are out-of-order messages that can now be sent, send them
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
  console.log('Websocket connection closed');
};

connection.open();
