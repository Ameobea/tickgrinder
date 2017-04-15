//! Creates a list of all running data downloads including information about the downloads' progress, the downloader that started them, etc.

const _ = require('lodash');
import React from 'react';
import { connect } from 'dva';
import { Table, Tooltip, Progress } from 'antd';

import { InstanceShape, HistTickDstShape } from '../../utils/commands';
import { Instance } from '../instances/Instance';
import DownloadControl from './DownloadControl';

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
  dataIndex: 'start_time',
  key: 'start_time',
}, {
  title: 'End Time',
  dataIndex: 'end_time',
  key: 'end_time',
}, {
  title: 'Progress',
  dataIndex: 'progress',
  key: 'progress',
}, {
  title: 'Control',
  dataIndex: 'control',
  key: 'control',
}];

class RunningDownloads extends React.Component {
  constructor(props) {
    super(props);
    // send a query to all running instances to list their currently running downloads
    setTimeout(() => {
      // props.dispatch({type: 'data/getRunningDownloads'});
    }, 2000);
  }

  // shouldComponentUpdate(nextProps, nextState) {
  //   if(_.isEqual(nextProps.runningDownloads, this.props.runningDownloads)) {
  //     return false;
  //   }

  //   return true;
  // }

  render() {
    let {dispatch, runningDownloads} = this.props;
    // map the list of running downloads to a data source for the table.  Converts instance data into
    // a pretty rendered instance like those on the instance management page.  Adds buttons for controlling running downloads.
    let dataSource = runningDownloads.map(download => {
      // if the list is empty, send out a query for the status of the download
      dispatch({
        type: 'data/getDownloadProgress',
        downloaderUuid: download.downloader.uuid,
        downloadId: download.id
      });
      let curTime = download.cur_time;
      if(curTime === undefined || curTime === null) {
        curTime = download.start_time;
      }
      let renderedInstance = <Instance instance_type={download.downloader.instance_type} uuid={download.downloader.uuid} />;

      return {...download,
        key: download.id,
        downloader: renderedInstance,
        control: <DownloadControl downloadId={download.id} downloaderId={download.downloader.uuid} />,
        progress: <DownloadProgress curTime={curTime} endTime={download.end_time} startTime={download.start_time} />
      };
    });

    return <Table columns={columns} dataSource={dataSource} />;
  }
}

RunningDownloads.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  runningDownloads: React.PropTypes.arrayOf(React.PropTypes.shape({
    // TODO: Create a `DataDownloadShape` and move this to there
    id: React.PropTypes.string.isRequired,
    downloader: React.PropTypes.shape(InstanceShape).isRequired,
    start_time: React.PropTypes.number.isRequired,
    cur_time: React.PropTypes.number,
    end_time: React.PropTypes.number.isRequired,
    symbol: React.PropTypes.string.isRequired,
    dst: React.PropTypes.shape(HistTickDstShape).isRequired,
  })).isRequired,
};

/**
 * Creates a progress guage for a runnning data download.
 */
const DownloadProgress = ({curTime, startTime, endTime}) => {
  let percent;
  let tooltip;
  if(endTime !== 0) {
    percent = curTime / (endTime - startTime);
    tooltip = `Current download progress: ${percent}`
  } else {
    percent = 100;
    tooltip = 'Downloading live data...'
  }
  return (
    <Tooltip title={tooltip}>
      <Progress percent={percent} showInfo={false} status='active' />
    </Tooltip>
  );
};

DownloadProgress.propTypes = {
  curTime: React.PropTypes.number.isRequired,
  endTime: React.PropTypes.number.isRequired,
  startTime: React.PropTypes.number.isRequired,
};

function mapProps(state) {
  return {
    runningDownloads: state.data.runningDownloads,
  };
}

export default connect(mapProps)(RunningDownloads);
