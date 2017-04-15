//! Downloads historical data from the Poloniex HTTP API endpoints, passing it through executors and into sinks.
// @flow

const _ = require('lodash');
const assert = require('assert');
const https = require('https');

import util from 'tickgrinder_util';
const { Tickstream, Log, POLONIEX_NEW_TRADE } = util.ffi;
const CONF = require('./conf');

type PoloniexRawTrade = {globalTradeID: number, tradeID: number, date: string, type: string, rate: string, amount: string, total: string};
type ExecutorDescriptor = {id: number, pointer: any};

const SECONDS_IN_AN_HOUR = 3600;
const SECONDS_IN_A_YEAR = 31556926;

/**
 * Initializes a download of historical data for a pair.  This will attempt to fetch all of the stored data over that time period and
 * write it to the sink.  This will internally manage the API limitations.  Start/end timestamps are Unix format second precision.
 */
function initHistTradeDownload(
  pair: string, startTimestamp: number, endTimestamp: number, outputPath: string, ourUuid: string, downloadComplete: () => void,
  progressUpdated: (curTimestamp: number) => void, cs: any
) {
  // create an executor to which to funnel the data
  let executor: ExecutorDescriptor = Tickstream.getCsvSinkExecutor(POLONIEX_NEW_TRADE, outputPath);

  // the start point of the current download segment's query
  let curStartTimestamp = startTimestamp;
  // true if we're currently working our way back in time in a large segment
  let areBacktracking = false;

  // make sure that we're requesting less than a year's worth of trades to start off
  let curEndTimestamp;
  if((endTimestamp - curStartTimestamp) > SECONDS_IN_A_YEAR) {
    curEndTimestamp = curStartTimestamp + (SECONDS_IN_A_YEAR * .99);
  } else {
    curEndTimestamp = endTimestamp;
  }

  // the most recent ticks that have been downloaded.  Since the segment's stop point is reduced until it is all downloaded,
  // this value is used to determine where to start the next segment once it's completely downloaded.
  let maxEndTimestamp = curEndTimestamp;

  function downloadChunk() {
    Log.debug(
      cs,
      `Hist. Download ${pair}`,
      `Downloading chunk from ${curStartTimestamp} : ${curEndTimestamp}`
    );
    fetchTradeHistory(pair, curStartTimestamp, curEndTimestamp, cs).then((data: Array<PoloniexRawTrade>) => {
      // sort the trades from oldest to newest
      let sortedData = _.sortBy(data, trade => trade.tradeID);
      if(data.length > 0)
        assert(_.last(sortedData).tradeID > sortedData[0].tradeID);

      // process the trades into the sink
      for(var i=0; i<sortedData.length; i++) {
        // convert the date from "2017-03-10 01:31:08" format into a Unix timestamp in the GMT time zone
        let timestamp = new Date(sortedData[i].date + " GMT").getTime();
        // convert the numeric values to strings because that's what the Rust parser expects
        sortedData[i].tradeID = `${sortedData[i].tradeID}`;
        sortedData[i].globalTradeID = `${sortedData[i].globalTradeID}`;
        // send the trade through the executor and into the sink
        Tickstream.executorExec(executor, timestamp, JSON.stringify(sortedData[i]));
      }

      // update download progress
      progressUpdated(curStartTimestamp);

      if(maxEndTimestamp < curEndTimestamp) {
        Log.debug(cs, '', `New \`maxEndTimestamp\` set: ${maxEndTimestamp}`);
        maxEndTimestamp = curEndTimestamp;
      }

      if(sortedData.length === 50000) {
        // if it was more than 50,000 trades, download what's missing before going on
        curEndTimestamp = Math.round(new Date(sortedData[0].date + " GMT").getTime() / 1000) - 1;
        // if this is the first attempt to download an oversized segment, update `maxEndTimestamp`
        if(!areBacktracking){
          Log.debug(cs, '', 'We weren\'t backtracking but now are due to hitting a max-sized result');
          Log.debug(cs, '', `curStartTimestamp: ${curStartTimestamp}, new curEndTimestamp: ${curEndTimestamp}`);
          areBacktracking = true;
        }
      } else {
        // if we're backtracking and hit this code, it means we've finished the oversized segment and can move on.
        if(areBacktracking) {
          Log.debug(cs, '', `We were backtracking but hit a result with size ${sortedData.length}.`);
          Log.debug(cs, '', `Setting start timestamp to after previous max end timestamp: ${maxEndTimestamp}`);
          curStartTimestamp = maxEndTimestamp + 1;
          areBacktracking = false;
        } else {
          Log.debug(cs, '', 'Not currently backtracking and hit non-full block; downloading next segment.');
          curStartTimestamp = curEndTimestamp + 1;
        }

        // if less than 50,000 trades, then download the next segment
        if(endTimestamp - curEndTimestamp > SECONDS_IN_A_YEAR) {
          Log.debug(cs, '', 'More than a year\'s worth of data remaining before end; setting next segment size to one year after `curStartTimestamp`.');
          curEndTimestamp = curStartTimestamp + (SECONDS_IN_A_YEAR * .99);
        } else if(curEndTimestamp >= endTimestamp || curStartTimestamp >= endTimestamp) {
          Log.debug(cs, '', 'Download complete!');
          return downloadComplete();
        } else {
          Log.debug(cs, '', 'Less than a year remaining after current download and end; queueing up final block...');
          curEndTimestamp = endTimestamp;
        }
      }

      // download the next chunk after waiting a few seconds as to avoid overloading their API
      setTimeout(() => {
        downloadChunk();
      }, 2587); // TODO: make config value
    }).catch(e => {
      console.log(e);
    });
  }

  // call the recursive chunk download function and start the download progress
  downloadChunk();
}

/**
 * Queries the public API to return the last [count] trades for a given pair.  Results are limited to 50,000 trades and the supplied
 * window must be less that one year in size.  Trades are returned in reverse chronological order.  If there are over 50,000 trades in
 * the supplied result, the oldest trades will be truncated.
 * @param {string} pair - The pair of trade history to download formatted like "BTC_XMR"
 * @param {number} startTimestamp - The Unix timestamp of the start of the download window, second precision.
 * @param {number} endTimestamp - The Unix timestamp of the end of the download window, second precision.
 */
function fetchTradeHistory(pair: string, startTimestamp: number, endTimestamp: number, cs: any)
  :Promise<Array<PoloniexRawTrade>>
{
  return new Promise((fulfill, reject) => {
    https.get(`${CONF.poloniex_http_api_url}?command=returnTradeHistory&currencyPair=${pair}&start=${startTimestamp}&end=${endTimestamp}`, res => {
      res.setEncoding('utf8');
      let rawData = '';

      res.on('data', d => {
        rawData += d;
      }).on('error', e => {
        Log.error(cs, '', `Unable to feth historical trade data for pair ${pair}: ${e}`);
        reject(e);
      });

      res.on('end', () => {
        try {
          let parsed: Array<PoloniexRawTrade> = JSON.parse(rawData);
          fulfill(parsed);
        } catch(e) {
          Log.error(cs, '', `Error while parsing JSON response from Poloniex API during historical trade fetching: ${e}`);
          reject(e);
        }
      });
    });
  });
}

module.exports = initHistTradeDownload;
