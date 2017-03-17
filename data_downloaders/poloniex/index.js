//! Data downloader that connects to the Poloniex Websocket API to record live streaming DOM data
// @flow

const CONF = require('./src/conf');
import util from 'tickgrinder_util';
const { Log } = util.ffi;
const { v4, initWs } = util;
const startWsDownload = require('./src/streaming');
const initHistTradeDownload = require('./src/historical');

type RunningDownload = {
  id: string,
  symbol: string,
  downloader: {
    instance_type: string,
    uuid: string
  },
  start_time: number,
  cur_time: number,
  end_time: number,
  dst: any
};

// holds all active downloads and their progress, used to respond to commands requesting their progress
var runningDownloads: { [key: string]: RunningDownload } = {};

// set up some state for communicating with the platform's util library through FFI.
let cs = Log.get_cs('Poloniex Data Downloader');

// usage: ./run.sh uuid
let ourUuid: string = process.argv[2];

if(!ourUuid) {
  console.error('Usage: node manager.js uuid');
  process.exit(0);
} else {
  Log.notice(cs, '', `Poloniex Data Downloader now listening for commands on ${CONF.redis_control_channel} and ${ourUuid}`);
}

const ourInstance = {
  instance_type: 'Poloniex Data Downloader',
  uuid: ourUuid,
};

// set up Websocket connection to the platform's messaging system
let socket: WebSocket = initWs(handleWsMessage, null, ourUuid, wsError);
socket.on('open', () => {
  // send ready message to notify the platform that we're up and running
  let msgUuid = v4();
  let msg = {Ready: {instance_type: 'Poloniex Data Downloader', uuid: ourUuid}};
  let wsmsg = {uuid: msgUuid, channel: CONF.redis_control_channel, message: JSON.stringify({uuid: msgUuid, cmd: msg})};
  socket.send(JSON.stringify(wsmsg));
});

function wsError(e: string) {
  Log.error(cs, 'Websocket', `Error in WebSocket connection: ${e}`);
}

/**
 * Given a command or a response from the platform, determines if an action needs to be taken and, if it does, takes it.
 * Also sends back responses conditionally.
 */
function handleWsMessage(dispatch: any, raw_msg: {uuid: string, channel: string, message: string}) {
  let msg = JSON.parse(raw_msg.message);
  if(msg.cmd) {
    // ignore log messages
    if(msg.cmd.Log) {
      return;
    }
    let res = handleCommand(msg.cmd);
    if(res) {
      // get a response to send back and send it
      let wsMsg = {uuid: msg.uuid, channel: CONF.redis_responses_channel, message: JSON.stringify({uuid: msg.uuid, res: res})};
      socket.send(JSON.stringify(wsMsg));
    }
  } else if(msg.res) {
    // We don't really need to listen for any responses
    return;
  } else {
    Log.error(cs, 'Platform Communication', `Received a message without a \`cmd\` or \`res\`: ${JSON.stringify(msg)}`);
  }
}

/**
 * Given a command send to our instance, executes it (if it needs to be executed) and optionally returns a response to be sent back.
 */
function handleCommand(cmd: any): any {
  if(cmd == 'Ping') {
    return {Pong: {args: [ourUuid]}};
  } else if(cmd == 'Type') {
    return {Info: {info: 'Poloniex Data Downloader'}};
  } else if(cmd == 'Kill') {
    setTimeout(() => {
      console.log('Rushing B no stop.');
      Log.notice(cs, '', 'Poloniex Data Downloader is despawning.');
      process.exit(0);
    }, 3001);
    return {Info: {info: 'Poloniex Data Downloader is despawning in 3 seconds...'}};
  } else if(cmd.DownloadTicks) {
    // Pairs that look like "HIST_XMR_BTC" will be downloaded using the historical trades HTTP downloader
    // Pairs that look like "WS_XMR_BTC" will be downloaded using the streaming WebSocket downloader
    if(/HIST_.+/.test(cmd.DownloadTicks.symbol)) {
      if(!cmd.DownloadTicks.dst.Flatfile) {
        return {Error: {status: 'Unacceptable tick destination; this downloader only works with the `Flatfile` downloader.'}};
      }
      let outputFilename = cmd.DownloadTicks.dst.Flatfile.filename;
      // convert start/end times from nanoseconds to seconds
      let startTimestamp = Math.floor(cmd.DownloadTicks.start_time / (1000 * 1000 * 1000));
      let endTimestamp = Math.ceil(cmd.DownloadTicks.end_time / (1000 * 1000 * 1000));
      const downloadUuid = v4();
      setTimeout(() => {
        // register this download as in-progress
        runningDownloads[downloadUuid] = {
          id: downloadUuid,
          downloader: ourInstance,
          symbol: cmd.DownloadTicks.symbol,
          start_time: startTimestamp,
          cur_time: startTimestamp,
          end_time: endTimestamp,
          dst: cmd.DownloadTicks.dst,
        };

        // send download started message
        sendDownloadStartedMessage(runningDownloads[downloadUuid]);

        // function to be called once the download finishes
        const downloadComplete = () => {
          // send download complete message
          const msgUuid = v4();
          const wsMsg = {uuid: msgUuid, channel: CONF.redis_control_channel, message: {uuid: msgUuid, cmd: {DownloadComplete: {download: runningDownloads[downloadUuid]}}}};
          socket.send(JSON.stringify(wsMsg));

          // remove the running download from the running downloads list
          delete runningDownloads[downloadUuid];
        };

        // function that can be called by the downloader to indicate that it has made progress during the download
        const progressUpdated = (curTimestamp: number) => {
          runningDownloads[downloadUuid].cur_time = curTimestamp;
        };

        // start the download
        initHistTradeDownload(cmd.DownloadTicks.symbol.split('HIST_')[1], startTimestamp, endTimestamp, outputFilename, ourUuid, downloadComplete, progressUpdated, cs);
      }, 0);

      return 'Ok';
    } else if(/WS_.+/.test(cmd.DownloadTicks.symbol)) {
      const downloadUuid = v4();
      runningDownloads[downloadUuid] = {
        id: downloadUuid,
        downloader: ourInstance,
        symbol: cmd.DownloadTicks.symbol,
        start_time: 0,
        cur_time: 0,
        end_time: 0,
        dst: cmd.DownloadTicks.dst,
      };

      let pair = cmd.DownloadTicks.symbol.split('WS_')[1];
      const isDownloadCancelled = (): boolean => {
        return !runningDownloads[downloadUuid];
      };

      // send download started message
      sendDownloadStartedMessage(downloadUuid, cmd.DownloadTicks.symbol, 0, 0, cmd.DownloadTicks.dst, ourInstance);

      // this only works for `Flatfile` destinations for now.
      if(!cmd.DownloadTicks.dst.Flatfile) {
        return {Error: {status: 'Streaming Poloniex Downloader currently only works with the `Flatfile` hist tick destination!'}};
      }

      return startWsDownload(pair, cmd.DownloadTicks.dst, cs, isDownloadCancelled);
    } else {
      return {Error: {status: 'Malformed symbol received; must be in format \'HIST_BTC_XMR\' or \'WS_BTC_XMR\'.'}};
    }
  } else if(cmd == 'ListRunningDownloads') {
    let runningList = [];
    for(let downloadId in runningDownloads) {
      runningList.push(runningDownloads[downloadId]);
    }
    return {RunningDownloads: {downloads: runningList}};
  } else if(cmd.CancelDataDownload) {
    if(runningDownloads[cmd.CancelDataDownload.id]) {
      // only able to cancel streaming downloads, not historical downloads
      if(/WS_.+/.test(runningDownloads[cmd.CancelDataDownload.id].symbol)) {
        delete runningDownloads[cmd.CancelDataDownload.id];
      } else {
        return {Error: {status: 'Unable to cancel historical data download!'}};
      }
    } else {
      return {Error: {status: 'There are no currently running downloads with that UUID!'}};
    }
  } else if(cmd.GetDownloadProgress) {
    let id = cmd.GetDownloadProgress.id;
    if(runningDownloads[id]) {
      let runningDownload = runningDownloads[id];
      runningDownload.id = id;
      return {DownloadProgress: {download: runningDownload}};
    } else {
      return {Error: {status: 'There are no currently running downloads with that UUID!'}};
    }
  } else {
    return {Error: {status: `Poloniex Data Downloader doesn\'t recognize that command: ${JSON.stringify(cmd)}`}};
  }
}

/**
 * Broadcasts a command to the platform indicating that a data download has started.
 */
function sendDownloadStartedMessage(download: RunningDownload) {
  const msgUuid = v4();
  const msg = {uuid: msgUuid, cmd: {DownloadStarted: {download: download}}};
  const wsmsg = {uuid: msgUuid, channel: CONF.redis_control_channel, message: JSON.stringify(msg)};
  socket.send(JSON.stringify(wsmsg));
}
