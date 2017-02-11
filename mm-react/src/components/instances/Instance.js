//! One instance as displayed in `LiveInstances`.  Contains information about the instance and controls.

import { connect } from 'dva';
import { Tooltip, Button, Icon, Popconfirm } from 'antd';

import styles from '../../static/css/instances.css';

/// Sends a kill message to an instance
const killInstance = (dispatch, uuid) => {
  dispatch({type: 'platform_communication/sendCommand', uuid: uuid, channel: uuid, cb_action: 'instances/instanceKillMessageReceived'});
};

const Instance = ({dispatch, instance_type, uuid}) => {
  return (
    // TODO: Button shortcut to show log messages from this instance
    // TODO: Button to stop this instance
    <Tooltip title={uuid}>
      <span className={styles.instance}>
        {instance_type}
        <Popconfirm
          placement="rightTop"
          title="Really kill this instance?"
          onConfirm={() => killInstance(dispatch, uuid)}
          okText="Yes"
          cancelText="Cancel"
        >
          <Button type="primary" icon="close" />
        </Popconfirm>
      </span>
    </Tooltip>
  );
}

export default {
  Instance: connect()(Instance),
  killInstance: killInstance,
}
