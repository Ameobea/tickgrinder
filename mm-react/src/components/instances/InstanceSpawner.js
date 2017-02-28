//! Dialog to allow users to spawn new instances.

import { connect } from 'dva';
import React from 'react';
import { message, Tooltip, Icon, Select, Button } from 'antd';
const Option = Select.Option;

import { getInstance, InstanceShape } from '../../utils/commands';
import MacroManager from '../MacroManager';
import styles from '../../static/css/instances.css';

const MacroInfo = () => {
  return (
    <Tooltip title="Click here for info on spawner macros">
      <Icon className={styles.infoTooltip} type="question" />
    </Tooltip>
  );
};

const spawnableInstances = [
  {name: 'Backtester', cmd: 'SpawnBacktester'},
  {name: 'Logger', cmd: 'SpawnLogger'},
  {name: 'Tick Parser', cmd: 'SpawnTickParser'},
  {name: 'Optimizer', cmd: 'SpawnOptimizer'},
  {name: 'Redis Server', cmd: 'SpawnRedisServer'},
  {name: 'FXCM Data Downloader', cmd: 'SpawnFxcmDataDownloader'},
];

const spawnChanged = (val, opt, dispatch) => {
  dispatch({type: 'instances/instanceSpawnChanged', cmd: val, name: opt.props.children});
};

const spawnButtonClicked = (dispatch, living_instances, {name, cmd}) => {
  let spawner_uuid = getInstance('Spawner', living_instances);
  if(spawner_uuid.length === 0) {
    // if no living spawner in the census list, display error message and return
    message.error('No Spawner instance was detected running on the platform; unable to spawn instance!');
    return;
  }

  let cb_action = 'instances/instanceSpawnCallback';
  dispatch({type: 'platform_communication/sendCommand', channel: spawner_uuid[0].uuid, cmd: cmd, cb_action: cb_action});
};

const SingleSpawner = ({dispatch, living_instances, spawn_opt}) => {
  let options = [];
  for(var i=0; i<spawnableInstances.length; i++) {
    let opt = (
      <Option key={i} value={spawnableInstances[i].cmd} >
        {spawnableInstances[i].name}
      </Option>
    );
    options.push(opt);
  }

  const handleSelect = (val, opt) => spawnChanged(val, opt, dispatch);
  const handleClick = () => spawnButtonClicked(dispatch, living_instances, spawn_opt);

  return (
    <div className={styles.singleSpawner}>
      <Select
        defaultValue={spawn_opt.cmd}
        onSelect={handleSelect}
        style={{ width: 160 }}
      >
        {options}
      </Select>
      <Button onClick={handleClick} type='primary'>
        {'Spawn Instance'}
      </Button>
    </div>
  );
};

SingleSpawner.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  living_instances: React.PropTypes.arrayOf(React.PropTypes.shape(InstanceShape)).isRequired,
  spawn_opt: React.PropTypes.shape({name: React.PropTypes.string.isRequired, cmd: React.PropTypes.any}).isRequired,
};

const InstanceSpawner = ({dispatch, living_instances, spawn_opt}) => {
  return (
    <div className={styles.instanceSpawner}>
      <div className={styles.header}>
        {'Spawn a Single Instance'}
      </div>
      <SingleSpawner
        dispatch={dispatch}
        living_instances={living_instances}
        spawn_opt={spawn_opt}
      />

      <div className={styles.header}>
        {'Execute a Spawner Macro'}
        <MacroInfo />
      </div>
      <MacroManager />
    </div>
  );
};

InstanceSpawner.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  living_instances: React.PropTypes.arrayOf(React.PropTypes.shape(InstanceShape)).isRequired,
  spawn_opt: React.PropTypes.shape({name: React.PropTypes.string.isRequired, cmd: React.PropTypes.any}).isRequired,
};

function mapProps(state) {
  return {
    living_instances: state.instances.living_instances,
    spawn_opt: state.instances.selected_spawn_opt,
  };
}

export default connect(mapProps)(InstanceSpawner);
