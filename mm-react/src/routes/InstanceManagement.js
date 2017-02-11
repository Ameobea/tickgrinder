import React from 'react';

import { connect } from 'dva';

import ContentContainer from '../components/ContentContainer';
import LiveInstances from '../components/instances/LiveInstances';
import InstanceSpawner from '../components/instances/InstanceSpawner';
import styles from '../static/css/instances.css';

function mapProps(state) {
  return {
    instances: state.instances.living_instances,
  };
}

const InstanceManagement = connect(mapProps)(({dispatch, instances}) => {
  return (
    <div className={styles.instanceManagement}>
      <LiveInstances instances={instances} />
      <InstanceSpawner />
    </div>
  );
});

export default () => { return (
  <ContentContainer title="Instance Management">
    <InstanceManagement />
  </ContentContainer>
);}
