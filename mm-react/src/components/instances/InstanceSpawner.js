//! Dialog to allow users to spawn new instances.

import { connect } from 'dva';
import { Tooltip, Icon } from 'antd';

import styles from '../../static/css/instances.css';

const MacroInfo = () => {
  return (
    <Tooltip title="See TODO TODO TODO for info on spawner macros">
      <Icon type="question" className={styles.infoTooltip} />
    </Tooltip>
  );
}

const InstanceSpawner = ({dispatch}) => {
  return (
    <div className={styles.instanceSpawner}>
      <div className={styles.header}>Spawn a Single Instance</div>

      <div className={styles.header}>Execute a Spawner Macro<MacroInfo /></div>
    </div>
  );
};

export default connect()(InstanceSpawner);
