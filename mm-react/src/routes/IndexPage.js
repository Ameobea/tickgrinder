import React from 'react';
import { connect } from 'dva';
import styles from './IndexPage.css';
import { Switch } from 'antd';

function IndexPage({instances}) {
  return (
    <div className={styles.normal}>
      <div className={styles.title}><h1>TickGrinder Dashboard</h1></div>
      <div className={styles.instances}>{instances}</div>
      <Switch />
    </div>
  );
}

IndexPage.propTypes = {
};

function mapState(state) {
  return { title: state.title };
}
export default connect(mapState)(IndexPage);
