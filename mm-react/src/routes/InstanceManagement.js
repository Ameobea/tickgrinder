import React from 'react';

import { connect } from 'dva';

import ContentContainer from '../components/ContentContainer';
import LiveInstances from '../components/instances/LiveInstances';

function mapProps(state) {
  return {
    instances: state.instances.living_instances,
  };
}

const InstanceManagement = connect(mapProps)(({dispatch, instances}) => {
  return (
    <LiveInstances instances={instances} />
  );
});

export default () => { return (
  <ContentContainer title="Instance Management">
    <InstanceManagement />
  </ContentContainer>
);}
