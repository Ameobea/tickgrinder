//! A dynamically updated view of all running instances in the platform with controls for spawning new
//! instances, killing old instances, and monitoring platform status.

import { connect } from 'dva';

import Instance from './Instance';

const LiveInstances = ({dispatch, instances}) {
  let insts = [];
  for(var i=0; i<instances.length; i++) {
    let inst = instances[i];
    insts.push(<Instance uuid={inst.uuid} instance_type={inst.instance_type} />)
  }

  return (

  );
}

function mapProps(state) {
  return {
    instances: state.instances.living_instances,
  };
}

export default connect(mapProps)(LiveInstances);
