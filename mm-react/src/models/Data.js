//! Handles state related to data management and downloading for the MM.

import { message } from 'antd';
import { select } from 'redux-saga/effects';

import { dataDownloaders } from '../utils/const';

export default {
  namespace: 'data',

  state: {
    runningDownloads: [], // all actively running data downloads
    downloadedData: [], // contains information about data that the platform has stored
    downloadProgresses: [], // contains the progress of all running backtests
    dataDownloadSettings: {
      startTime: false,
      endTime: false,
      symbol: false,
      downloaderId: false,
      tickDst: false,
    },
  },

  reducers: {
    /**
     * Called whenever the user changes his or her selection of settings for the download.  Only takes into
     * account the setting that was changed.
     */
    downloadSettingChanged (state, {startTime, endTime, symbol, downloaderId}) {
      if(startTime) {
        return{...state,
          dataDownloadSettings: {...state.dataDownloadSettings,
            startTime: startTime,
          },
        };
      } else if(startTime) {
        return{...state,
          dataDownloadSettings: {...state.dataDownloadSettings,
            startTime: startTime,
          },
        };
      } else if(symbol) {
        return{...state,
          dataDownloadSettings: {...state.dataDownloadSettings,
            symbol: symbol,
          },
        };
      } else if(downloaderId) {
        return{...state,
          dataDownloadSettings: {...state.dataDownloadSettings,
            downloaderId: downloaderId,
          },
        };
      }
    },

    /**
     * Called when the spawner responds to a request to spawn a data downloader
     */
    dataDownloaderSpawnResponseReceived (state, {msg}) {
      if(msg.res == 'Ok') {
        message.success('Data downloader spawn request accepted');
      } else if(msg.res.Error) {
        message.error('Error when attempting to spawn data downloader: ' + msg.res.Error.status);
      } else {
        message.error('Unexpected response received from spawner when attempting to spawn data downloader: ' + JSON.stringify(msg));
      }

      return {...state};
    },

    /**
     * Called as a callback for responses received from commands dispatched to start data downloads.
     * If the download started successfully, adds the download to the list of running downloads and displays
     * a message.  If unsuccessful, displays an error message.
     */
    downloadStarted (state, {msg}) {
      if(msg.res.DownloadStarted) {
        message.loading(`Data download for symbol ${msg.DownloadStarted.symbol} has been successfully initialized.`);
        return {...state,
          runningDownloads: [...state.runningDownloads, msg.res.DownloadStarted],
        };
      } else if(msg.res.Error) {
        message.error('Error starting data download: ' + msg.res.Error.status);
      } else {
        message.error('Received unexpected response to data download request: ' + JSON.stringify(msg));
      }

      // TODO: Remove the download request from the list of pending download requests
      return {...state};
    },

    /**
     * Called as a callback for `DownloadComplete` commands send by data downloaders.  Removes the download from
     * the list of running downloads and displays a message indicating its completion.
     */
    downloadFinished (state, {msg}) {
      // display a notification of the download's success
      let {symbol, id, start_time, end_time} = msg.DownloadComplete;
      message.success(`Data download for symbol ${symbol} with ID ${id} has completed after ${end_time - start_time} seconds!`);

      return {...state,
        // remove the completed download from the list
        runningDownloads: [state.runningDownloads.filter(download => download.id !== id)]
      };
    },

    /**
     * Called as a callback for `GetDownloadProgress` commands.
     */
    downloadProgressReceived (state, {msg}) {
      if(msg.res.DownloadProgress) {
        // remove the old progress from the state (if it exists) and put the new progress in
        let newProgresses = state.downloadProgresses.filter(prog => prog.id !== msg.res.DownloadProgress.id);
        newProgresses.push(msg.res.DownloadProgress);

        return {...state,
          downloadProgresses: newProgresses,
        };
      } else if(msg.res.Error) {
        message.error(`Received error when requesting progress of data download: ${msg.res.Error.status}`);
      } else {
        message.error(`Received unexpected response when requesting progress of data download: ${JSON.stringify(msg)}`);
      }

      return {...state};
    }
  },

  effects: {
    /**
     * Sends a command to the Spawner instance to spawn a data downloader of the specified type
     */
    *spawnDataDownloader ({downloaderName}, {call, put}) {
      // get the proper command to spawn the downloader of the specified type
      let cmd = false;
      for(var i=0; i<dataDownloaders.length; i++) {
        if(dataDownloaders[i].name == downloaderName) {
          cmd = dataDownloaders[i].cmd;
        }
      }

      yield put({
        cb_action: 'data/dataDownloaderSpawnResponseReceived',
        cmd: cmd,
        instance_name: 'Spawner',
        type: 'platform_communication/sendCommandToInstance',
      });
    },

    /**
     * Dispatches the correct command to initialize the specified data download.
     */
    *startDataDownload ({}, {call, put}) {
      // get the currently selected download settings and make sure that they're all set
      let downloadSettings = yield select(gstate => gstate.data.dataDownloadSettings);
      if(downloadSettings.startTime === false || downloadSettings.endTime === false || downloadSettings.symbol === false ||
         downloadSettings.downloaderId === false || downloadSettings.tickDst === false) {
        message.error('You must set all the download settings before initializing a data downoad');
        return;
      }

      let cmd = {DownloadTicks: {
        start_time: downloadSettings.startTime,
        end_time: downloadSettings.endTime,
        symbol: downloadSettings.symbol,
        dst: downloadSettings.tickDst,
      }};

      yield put({
        type: 'instances/sendCommand',
        channel: downloadSettings.downloaderId,
        cmd: cmd,
        cb_action: 'data/downloadStarted'
      });
      // TODO
    },

    /**
     * Sends a request to get the progress of the current download.
    */
    *getDownloadProgress ({downloaderUuid, downloadId}, {call, put}) {
      let cmd = {GetDownloadProgress: {id: downloadId}};
      yield put({
        type: 'platform_communication/sendCommand',
        channel: downloaderUuid,
        cmd: cmd,
        cb_action: 'data/downloadProgressReceived'
      });
    }
  },
};
