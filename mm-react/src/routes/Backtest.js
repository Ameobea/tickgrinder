//! Backtest Management Interface

import React from 'react';
import { Link } from 'react-router'
import { connect } from 'dva';
import { Switch, DatePicker, Row, Col } from 'antd';

import wrapContent from '../components/ContentContainer';
import styles from '../static/css/globalStyle.css';

function backtestContent() {
  return (
    <div className={styles.normal}>
      Backtests
    </div>
  );
}

let wrapped = wrapContent("Backtest Management", backtestContent);

export default connect()(wrapped);
