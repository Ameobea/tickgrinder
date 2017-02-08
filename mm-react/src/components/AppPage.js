//! A page of the application.  Contains the basic structure of the GUI including title, header, navigation, and footer.

import React from 'react';
import { connect } from 'dva';
import { Layout } from 'antd';
const { Header, Content, Footer, Sider } = Layout;

import DashHeader from './DashHeader';
import gstyles from '../static/css/globalStyle.css';

class AppPage extends React.Component {
  render() {
    return (
      <Layout className={gstyles.application}>
        <DashHeader title={this.props.title} />
        <Content className={gstyles.content}>
          {this.props.children}
        </Content>
        <Footer style={{ textAlign: 'center' }}>
          TickGrinder Algorithmic Trading Platform; Created by Casey Primozic ©2017
        </Footer>
      </Layout>
    );
  }
}

function mapProps(state) {
  return { title: state.global.title };
}

export default connect(mapProps)(AppPage);
