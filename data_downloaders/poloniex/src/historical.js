//! Downloads historical data from the Poloniex HTTP API endpoints, passing it through executors and into sinks.
// @flow

const assert = require('assert');
const https = require('https');

import util from 'tickgrinder_util';
const { Tickstream, Log, POLONIEX_NEW_TRADE } = util.ffi;
const CONF = require('./conf');

type PoloniexRawTrade = {globtalTradeID: number, tradeID: number, date: string, type: string, rate: string, amount: string, total: string};
type ExecutorDescriptor = {id: number, pointer: any};

const SECONDS_IN_A_YEAR = 31556926;

/**
 * Initializes a download of historical data for a pair.  This will attempt to fetch all of the stored data over that time period and
 * write it to the sink.  This will internally manage the API limitations.  Start/end timestamps are Unix format second precision.
 */
function initHistTradeDownload(pair: string, startTimestamp: number, endTimestamp: number, outputPath: string, ourUuid: string, downloadComplete: () => void) {
  // create an executor to which to funnel the data
  let executor: ExecutorDescriptor = Tickstream.getCsvSinkExecutor(POLONIEX_NEW_TRADE, outputPath);

  let curStartTimestamp = startTimestamp;
  let curEndTimestamp = endTimestamp;
  // make sure that we're requesting less than a year's worth of trades to start off
  if(endTimestamp - curStartTimestamp > SECONDS_IN_A_YEAR) {
    curEndTimestamp = curStartTimestamp + (SECONDS_IN_A_YEAR - 1001);
  } else {
    curEndTimestamp = endTimestamp;
  }

  function downloadChunk() {
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

      if(sortedData.length === 50000) {
        // if it was more than 50,000 trades, download what's missing before going on
        curEndTimestamp = new Date(sortedData[0].date).getTime() - 1;
      } else {
        curStartTimestamp = curEndTimestamp + 1;
        // if less than 50,000 trades, then download the next segment
        if(endTimestamp - curEndTimestamp > SECONDS_IN_A_YEAR) {
          curEndTimestamp = curStartTimestamp + (SECONDS_IN_A_YEAR - 1001);
        } else if(curEndTimestamp >= endTimestamp) {
          return downloadComplete();
        } else {
          curEndTimestamp = endTimestamp;
        }
      }

      // download the next chunk after waiting a few seconds as to avoid overloading their API
      setTimeout(() => {
        downloadChunk();
      }, 7683);
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

module.exports = initHistTradeDownload;
