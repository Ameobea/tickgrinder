//! Tool used to facilitate the downloading and saving of historical data or data from other sources
//! in general.  Will show on the `Data` page of the MM.

import React from 'react';
import { connect } from 'dva';
import { Select, Table, Button, Popconfirm, Tooltip, Progress } from 'antd';
const Option = Select.Option;

import { InstanceShape, HistTickDstShape } from '../../utils/commands';
import { Instance } from '../instances/Instance';

/**
 * Given a list of running instances, returns the set of those instances that are data downloaders
 */
const getDownloaders = livingInstances => {
  // It is assumed that all data downloader instances will have the phrase "Data Downloader" in their instance type
  return livingInstances.filter(inst => inst.instance_type.toLowerCase().indexOf('data downloader') !== -1);
};

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

/**
 * Creates a progress guage for a runnning data download.
 */
const DownloadProgress = ({dispatch, curTime, startTime, endTime}) => {
  return (
    <Tooltip title={`Current download progress: ${curTime}`}>
      <Progress percent={curTime / (endTime - startTime)} />
    </Tooltip>
  );
};

DownloadProgress.propTypes = {
  curTime: React.PropTypes.number.isRequired,
  dispatch: React.PropTypes.func.isRequired,
  endTime: React.PropTypes.number.isRequired,
  startTime: React.PropTypes.number.isRequired,
};

// TODO: Auto-update progress of all running data downloads every few seconds

class DataDownloader extends React.Component {
  render() {
    // get a list of all available data downloaders from the platform
    let available_downloaders = getDownloaders(this.props.livingInstances); // TODO

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
      title: 'Progress',
      dataIndex: 'progress',
      key: 'progress',
    }, {
      title: 'Control',
      dataIndex: 'control',
      key: 'control',
    }];

    // map the list of running downloads to a data source for the table.  Converts instance data into
    // a pretty rendered instance like those on the instance management page.  Adds buttons for controlling running downloads.
    let dataSource = this.props.runningDownloads.map(download => {
      // get the progress of the current download
      let curProgressList = this.props.downloadProgresses.filter(prog => prog.id == download.id);
      let curTime;
      if(curProgressList.length === 0) {
        // if the list is empty, send out a query for the status of the download
        this.props.dispatch({
          type: 'data/getDownloadProgress',
          downloaderUuid: download.downloader.id,
          downloadId: download.id
        });
        curTime = download.start_time;
      } else {
        curTime = curProgressList[0].cur_time;
      }
      let renderedInstance = <Instance instance_tye={download.downloader.instance_type} uuid={download.downloader.uuid} />;

      return {...download,
        downloader: renderedInstance,
        control: <DownloadControl dispatch={this.props.dispatch} downloadId={download.id} />,
        progress: <DownloadProgress curTime={curTime} endTime={download.end_time} startTime={download.start_time} />
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
  }
}

DataDownloader.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  downloadProgresses: React.PropTypes.arrayOf(React.PropTypes.shape({
    cur_time: React.PropTypes.number.isRequired,
    end_time: React.PropTypes.number.isRequired,
    id: React.PropTypes.string.isRequired,
    start_time: React.PropTypes.number.isRequired,
  })).isRequired,
  livingInstances: React.PropTypes.arrayOf(InstanceShape).isRequired,
  runningDownloads: React.PropTypes.arrayOf(React.PropTypes.shape({
    // TODO: Create a `DataDownloadShape` and move this to there
    id: React.PropTypes.string.isRequired,
    downloader: React.PropTypes.shape(InstanceShape).isRequired,
    start_time: React.PropTypes.number.isRequired,
    end_time: React.PropTypes.number.isRequired,
    symbol: React.PropTypes.string.isRequired,
    dst: React.PropTypes.shape(HistTickDstShape).isRequired,
  })).isRequired,
};

function mapProps(state) {
  return {
    runningDownloads: state.data.runningDownloads,
    livingInstances: state.instances.living_instances,
    downloadProgresses: state.data.downloadProgresses,
  };
}

export default connect(mapProps)(DataDownloader);
