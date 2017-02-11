//! One instance as displayed in `LiveInstances`.  Contains information about the instance and controls.

import { connect } from 'dva';
import { Tooltip, Button, Icon, Popconfirm } from 'antd';

import styles from '../../static/css/instances.css';

/// Sends a kill message to an instance
const killInstance = (dispatch, uuid) => {
  dispatch({
    type: 'platform_communication/sendCommand',
    channel: uuid,
    cb_action: 'instances/instanceKillMessageReceived',
    cmd: "Kill",
  });
};

const Instance = ({dispatch, instance_type, uuid}) => {
  return (
    // TODO: Button shortcut to show log messages from this instance
    // TODO: Button to stop this instance
    <div style={{ "marginTop": "5px" }}>
      <span className={styles.instance}>
        <Tooltip title={uuid}>
          {instance_type}
        </Tooltip>
        <Popconfirm
          placement="rightTop"
          title="Really kill this instance?"
          onConfirm={() => killInstance(dispatch, uuid)}
          okText="Yes"
          cancelText="Cancel"
        >
          <Tooltip title="Kill Instance">
            <Button type="primary" icon="close" shape="circle" className={styles.killButton} />
          </Tooltip>
        </Popconfirm>
    </span>
    </div>
  );
}

export default {
  Instance: connect()(Instance),
  killInstance: killInstance,
}
