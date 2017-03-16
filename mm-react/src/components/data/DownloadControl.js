//! Controls for managing a single data download.

import React from 'react';
import { connect } from 'dva';
import { Button, Popconfirm } from 'antd';

/**
 * Returns a function that sends a command to a data downloader instance to cancel a running data download
 */
const handleClick = (dispatch, downloaderId, downloadId) => {
  return () => {
    let cmd = {CancelDataDownload: {download_id: downloadId}};
    dispatch({type: 'platform_communication/sendCommand', channel: downloaderId, cmd: cmd});
  };
};

/**
 * Creates a set of buttons for controlling a running data download
 */
const DownloadControl = ({dispatch, downloaderId, downloadId}) => {
  return (
    <div>
      <Popconfirm
        onConfirm={handleClick(dispatch, downloaderId, downloadId)}
        title='Are you sure you want to cancel this data download?'
      >
        <Button type='danger'>{'Stop Download'}</Button>
      </Popconfirm>
    </div>
  );
};

DownloadControl.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  downloadId: React.PropTypes.string.isRequired,
  downloaderId: React.PropTypes.string.isRequired,
};

export default connect()(DownloadControl);
