//! Handles state related to data management and downloading for the MM.

import { message } from 'antd';
import { select } from 'redux-saga/effects';

export default {
  namespace: 'data',

  state: {
    runningDownloads: [], // all actively running data downloads
    downloadedData: [], // contains information about data that the platform has stored
  },

  reducers: {
    /**
     * Called as a callback for responses received from commands dispatched to start data downloads.
     * If the download started successfully, adds the download to the list of running downloads and displays
     * a message.  If unsuccessful, displays an error message.
     */
    downloadStarted (state, {msg}) {
      if(msg.res.DataDownloadStarted) {
        // TODO
      } else if(msg.res.Error) {
        message.error('Error starting data download: ' + msg.res.Error.status);
      } else {
        message.error('Received unexpected response to data download request: ' + JSON.stringify(msg));
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
      yield put({type: 'instances/sendCommand', channel: downloaderUuid, cmd: cmd});
      // TODO
    },
  },
};
