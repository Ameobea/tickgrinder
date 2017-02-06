//! Front page of the dashboard

import React from 'react';
import { connect } from 'dva';

import { Switch, DatePicker, Row, Col } from 'antd';

import styles from './IndexPage.css';

function IndexPage() {
  return (
    <div className={styles.normal}>
      <div className={styles.instances}>.</div>
      <Switch />
      <DatePicker />
    </div>
  );
}

IndexPage.propTypes = {};

export default connect()(IndexPage);
