//! Handles state related to data management and downloading for the MM.

import { message } from 'antd';
import { select } from 'redux-saga/effects';

export default {
  namespace: 'data',

  state: {
    runningDownloads: [], // all actively running data downloads
    downloadedData: [], // contains information about data that the platform has stored
    downloadProgresses: [], // contains the progress of all running backtests
  },

  reducers: {
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

  /**
   * Dispatches the correct command to initialize the specified data download.
   */
  effects: {
    *startDataDownload ({downloaderUuid, symbol, metadata}, {call, put}) {
      // get the type of data downloader that the selected instance is
      let runningInstances = yield select(gstate => gstate.instances.living_instances);
      let downloaderInstance = runningInstances.filter(inst => inst.uuid == downloaderUuid);
      let downloaderName;
      if(downloaderInstance.length === 0) {
        message.error('Selected downloader instance is not in list of living instances: ' + downloaderUuid);
        return;
      } else {
        downloaderName = downloaderInstance[0].uuid;
      }

      let cmd;
      switch (downloaderName) {
        // TODO
      }
      yield put({
        type: 'instances/sendCommand',
        channel: downloaderUuid,
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
