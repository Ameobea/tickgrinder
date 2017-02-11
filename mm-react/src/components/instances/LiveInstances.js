//! A dynamically updated view of all running instances in the platform with controls for spawning new
//! instances, killing old instances, and monitoring platform status.

import { connect } from 'dva';
import { Row, Col } from 'antd';

import { Instance} from './Instance';
import styles from '../../static/css/instances.css';

const LiveInstances = ({dispatch, instances}) => {
  let header = <Row key="header"><Col span={24}><span>Instance Name</span></Col></Row>;
  let insts = [header];
  for(var i=0; i<instances.length; i++) {
    let inst = instances[i];
    insts.push(
      <Row key={inst.uuid}>
        <Col span={24}><Instance uuid={inst.uuid} instance_type={inst.instance_type} /></Col>
      </Row>
    );
  }

  return (
    <span className={styles.instances}>{insts}</span>
  );
}

function mapProps(state) {
  return {
    instances: state.instances.living_instances,
  };
}

export default connect(mapProps)(LiveInstances);
