//! Contains nav information as well as the title

import React from 'react';
import { Link } from 'react-router'
import { connect } from 'dva';
import { Layout, Menu } from 'antd';
const { Submenu } = Menu;
const { Header } = Layout;

import gstyles from '../static/css/globalStyle.css';

class DashHeader extends React.Component {
  propTypes: {
    title: string
  }

  render() {
    return (
      <Header className={gstyles.header}>
        <Menu className={gstyles.nav} mode="horizontal" defaultSelectedKeys={['index']} style={{ lineHeight: '64px' }}>
          <Menu.Item key="index"><Link to="/index">Home</Link></Menu.Item>
          <Menu.Item key="backtest"><Link to="/backtest">Backtest Management</Link></Menu.Item>
          <Menu.Item key="data"><Link to="/data">Data Management</Link></Menu.Item>
          <Menu.Item key="log"><Link to="/log">Logging + Monitoring</Link></Menu.Item>
        </Menu>
      </Header>
    );
  }
}

function mapState(state) {
  return { title: state.global.title }
}

export default connect(mapState)(DashHeader);
