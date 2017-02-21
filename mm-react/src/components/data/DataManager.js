//! Tool used to facilitate the management of stored data.  Includes functionality to list what data is currently
//! stored and apply transformations and analysis to it.

import React from 'react';
import { connect } from 'dva';
import { Table } from 'antd';

const DataManager = ({dispatch, downloadedData}) => {
  const columns = [{
    // TODO
  }];

  return (
    <div>
      <h3>Downloaded Data</h3>
      <Table columns={columns} dataSource={downloadedData} />
    </div>
  );
};

DataManager.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  downloadedData: React.PropTypes.any.isRequired, // TODO: Update to official schema once it's established
};

function mapProps(state) {
  return {
    downloadedData: state.data.downloadedData,
  };
}

export default connect(mapProps)(DataManager);
