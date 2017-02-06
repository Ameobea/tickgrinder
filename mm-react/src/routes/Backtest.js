//! Backtest Management Interface

import React from 'react';
import { Link } from 'react-router'
import { connect } from 'dva';
import { Switch, DatePicker, Row, Col } from 'antd';

import styles from '../static/css/globalStyle.css';

function BacktestPage({title}) {
  return (
    <div className={styles.normal}>
      Backtests
    </div>
  );
}

function mapState(state) {
  return { };
}

export default connect(mapState)(BacktestPage);
