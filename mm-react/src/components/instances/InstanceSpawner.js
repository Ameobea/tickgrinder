//! Dialog to allow users to spawn new instances.

import { connect } from 'dva';
import { message, Tooltip, Icon, Select, Button } from 'antd';
const Option = Select.Option;

const CONF = require('../../conf');
import styles from '../../static/css/instances.css';

const MacroInfo = () => {
  return (
    <Tooltip title="Click here for info on spawner macros">
      <Icon type="question" className={styles.infoTooltip} />
    </Tooltip>
  );
};

/// Given the list of all currently running instances, returns the UUIDs of all instances with the specified name.
/// Returns an empty list if there are no living instances with the specified name.
const getInstance = (name, living_instances) => {
  return living_instances.filter(inst => inst.instance_type == name);
};

const spawnable_instances = [
  {name: "Backtester", cmd: "SpawnBacktester"},
  {name: "Logger", cmd: "SpawnLogger"},
  {name: "Tick Parser", cmd: "SpawnTickParser"},
  {name: "Optimizer", cmd: "SpawnOptimizer"},
  {name: "Redis Server", cmd: "SpawnRedisServer"},
  {name: "FXCM Data Downloader", cmd: "SpawnFxcmDataDownloader"},
];

const spawnChanged = (val, opt, dispatch) => {
  dispatch({type: 'instances/instanceSpawnChanged', val: val, opt: opt});
};

const spawnButtonClicked = (dispatch, living_instances, {name, cmd}) => {
  let spawner_uuid = getInstance("Spawner", living_instances);
  if(spawner_uuid.length === 0) {
    // if no living spawner in the census list, display error message and return
    message.error("No Spawner instance was detected running on the platform; unable to spawn instance!");
    return;
  }

  let cb_action = 'instances/instanceSpawnCallback';
  dispatch({type: 'platform_communication/sendCommand', channel: spawner_uuid[0].uuid, cmd: cmd, cb_action: cb_action});
};

const SingleSpawner = ({dispatch, living_instances, spawn_opt}) => {
  let options = [];
  for(var i=0; i<spawnable_instances.length; i++) {
    let opt = <Option key={i} value={spawnable_instances[i].cmd}>{spawnable_instances[i].name}</Option>;
    options.push(opt)
  }

  return (
    <div className={styles.singleSpawner}>
      <Select
        style={{ width: 160 }}
        defaultValue={spawnable_instances[0].cmd}
        onSelect={(val, opt) => spawnChanged(val, opt, dispatch)}
      >
        {options}
      </Select>
      <Button type="primary" onClick={() => spawnButtonClicked(dispatch, living_instances, spawn_opt)}>
        Spawn Instance
      </Button>
    </div>
  );
}

const InstanceSpawner = ({dispatch, living_instances, spawn_opt}) => {
  return (
    <div className={styles.instanceSpawner}>
      <div className={styles.header}>Spawn a Single Instance</div>
      <SingleSpawner dispatch={dispatch} living_instances={living_instances} spawn_opt={spawn_opt} />

      <div className={styles.header}>Execute a Spawner Macro<MacroInfo /></div>
    </div>
  );
};

function mapProps(state) {
  return {
    living_instances: state.instances.living_instances,
    spawn_opt: state.instances.selected_spawn_opt,
  };
}

export default connect(mapProps)(InstanceSpawner);
