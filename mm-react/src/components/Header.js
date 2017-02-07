//! Contains nav information as well as the title

import React from 'react';
import { Link } from 'react-router'
import { connect } from 'dva';
import { Row, Col } from 'antd';

import styles from '../static/css/globalStyle.css';

class Header extends React.Component {
  propTypes: {
    title: string
  }

  render() {
    return (
      <div className={styles.header}>
        <div className={styles.title}><h1>{this.props.title}</h1></div>
        <Row className={styles.nav}>
          <Col span={6}><Link to="/index">Home</Link></Col>
          <Col span={6}><Link to="/backtest">Backtest Management</Link></Col>
          <Col span={6}><Link to="/data">Data Management</Link></Col>
          <Col span={6}><Link to="/log">Logging + Monitoring</Link></Col>
        </Row>
      </div>
    );
  }
}

function mapState(state) {
  return { title: state.global.title }
}

export default connect(mapState)(Header);
