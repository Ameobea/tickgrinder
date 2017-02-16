//! One instance as displayed in `LiveInstances`.  Contains information about the instance and controls.

import { connect } from 'dva';
import React from 'react';
import { Tooltip, Button, Popconfirm } from 'antd';

import styles from '../../static/css/instances.css';

/// Sends a kill message to an instance
const killInstance = (dispatch, uuid) => {
  dispatch({
    type: 'platform_communication/sendCommand',
    channel: uuid,
    cb_action: 'instances/instanceKillMessageReceived',
    cmd: 'Kill',
  });
};

const Instance = ({dispatch, instance_type, uuid}) => {
  const handleConfirm = () => killInstance(dispatch, uuid);

  return (
    // TODO: Button shortcut to show log messages from this instance
      <div style={{ 'marginTop': '5px' }}>
          <span className={styles.instance}>
              <Tooltip title={uuid}>
                  {instance_type}
              </Tooltip>
              <Popconfirm
                  cancelText='Cancel'
                  okText='Yes'
                  onConfirm={handleConfirm}
                  placement='rightTop'
                  title='Really kill this instance?'
              >
                  <Tooltip title='Kill Instance'>
                      <Button
                          className={styles.killButton}
                          icon='close'
                          shape='circle'
                          type='primary'
                      />
                  </Tooltip>
              </Popconfirm>
          </span>
      </div>
  );
};

Instance.propTypes = {
  dispatch: React.PropTypes.function.isRequired,
  instance_type: React.PropTypes.string.isRequired,
  uuid: React.PropTypes.string.isRequired,
};

export default {
  Instance: connect()(Instance),
  killInstance: killInstance,
};
