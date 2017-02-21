//! Tool used to facilitate the downloading and saving of historical data or data from other sources
//! in general.  Will show on the `Data` page of the MM.

import React from 'react';
import { connect } from 'dva';
import { Select, Table, Button } from 'antd';
const Option = Select.Option;

import { InstanceShape } from '../../utils/commands';
import { Instance } from '../instances/Instance';

/**
 * Creates a set of buttons for controlling a running data download
 */
const DownloadControl = ({dispatch, downloadId}) => {
  const handleClick = dispatch => {
    return () => {
      // TODO: Show popconfirm to ask if you really want to stop download and if yes, send command to do so
    };
  };
  return (
    <div>
      <Button onClick={handleClick(dispatch)} type='danger'>{'Stop Download'}</Button>
    </div>
  );
};

DownloadControl.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  downloadId: React.PropTypes.string.isRequired,
};

const DataDownloader = ({dispatch, runningDownloads}) => {
  // get a list of all available data downloaders from the platform
  let available_downloaders = []; // TODO

  // set up the running downloads table schema
  const columns = [{
    title: 'Downloader',
    dataIndex: 'downloader',
    key: 'downloader',
  }, {
    title: 'Symbol',
    dataIndex: 'symbol',
    key: 'symbol',
  }, {
    title: 'Start Time',
    dataIndex: 'startTime',
    key: 'startTime',
  }, {
    title: 'Control',
    dataIndex: 'control',
    key: 'control',
  }];

  // map the list of running downloads to a data source for the table.  Converts instance data into
  // a pretty rendered instance like those on the instance management page.  Adds buttons for controlling running downloads.
  let dataSource = runningDownloads.map(download => {
    let renderedInstance = <Instance instance_tye={download.downloader.instance_type} uuid={download.downloader.uuid} />;
    return {...download,
      downloader: renderedInstance,
      control: <DownloadControl dispatch={dispatch} downloadId={download.downloadId} />,
    };
  });

  return (
    <div>
      <h2>{'Start Data Download'}</h2>
      <Select>
        {available_downloaders}
      </Select>

      <h2>{'Running Downloads'}</h2>
      <Table columns={columns} dataSource={dataSource} />
    </div>
  );
};

DataDownloader.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  runningDownloads: React.PropTypes.arrayOf(React.PropTypes.shape({
    downloadId: React.PropTypes.string.isRequired,
    downloader: React.PropTypes.shape(InstanceShape).isRequired,
    startTime: React.PropTypes.string.isRequired,
    symbol: React.PropTypes.string,
    metadata: React.PropTypes.any,
  })).isRequired,
};

function mapProps(state) {
  return {
    runningDownloads: state.data.runningDownloads,
  };
}

export default connect(mapProps)(DataDownloader);
