//! Handles state related to data management and downloading for the MM.

import { message } from 'antd';
import { select } from 'redux-saga/effects';

import { dataDownloaders } from '../utils/data_util';

export default {
  namespace: 'data',

  state: {
    runningDownloads: [], // all actively running data downloads
    downloadedData: [], // contains information about data that the platform has stored
    downloadProgresses: [], // contains the progress of all running backtests
    dst: null, // the currently selected `HistTickDst` for a bactest (or null if the user has supplied incomplete/invalid params)
  },

  reducers: {
    /**
     * Called when the spawner responds to a request to spawn a data downloader
     */
    dataDownloaderSpawnResponseReceived(state, {msg}) {
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
     * Called when a data downloader sends a response to a command to start a data download.
     */
    downloadRequestResponseReceived(state, {msg}) {
      if(msg.res.Error) {
        message.error('Error when attempting to initialize data download: ' + msg.res.Error.status);
      } else if(msg.res !== 'Ok') {
        message.error(
          'Unexpected response received from data downloader instance when attempting to initialize data download: ' + JSON.stringify(msg)
        );
      }

      return {...state};
    },

    /**
     * Called when `DownloadStarted` commands are received. If the download started successfully, adds the download to the list of running
     * downloads and displays a message.  If unsuccessful, displays an error message.
     */
    downloadStarted(state, {msg}) {
      message.loading(`Data download for symbol ${msg.cmd.DownloadStarted.symbol} has been successfully initialized.`);
      return {...state,
        runningDownloads: [...state.runningDownloads, msg.cmd.DownloadStarted],
      };
    },

    /**
     * Called as a callback for `DownloadComplete` commands sent by data downloaders.  Removes the download from
     * the list of running downloads and displays a message indicating its completion.
     */
    downloadFinished(state, {msg}) {
      // display a notification of the download's success
      let {symbol, id, start_time, end_time} = msg.cmd.DownloadComplete;
      message.success(`Data download for symbol ${symbol} with ID ${id} has completed after ${end_time - start_time} seconds!`);

      return {...state,
        // remove the completed download from the list
        runningDownloads: [state.runningDownloads.filter(download => download.id !== id)]
      };
    },

    /**
     * Called as a callback for `GetDownloadProgress` commands.
     */
    downloadProgressReceived(state, {msg}) {
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
    },

    /**
     * Called when the user changes the params for a `TickSink` component; contains the new sink as a `HistTickSink`.
     */
    newDst(state, {dst}) {
      return {...state,
        dst: dst,
      };
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
