//! A page of the application.  Contains the basic structure of the GUI including title, header, navigation, and footer.

import React from 'react';
import { connect } from 'dva';

import Header from './Header';
import gstyles from '../static/css/globalStyle.css';

class AppPage extends React.Component {
  render() {
    return (
      <div className={gstyles.application}>
        <Header title={this.props.title} />
        <div className={gstyles.content}>
          {this.props.children}
        </div>
      </div>
    );
  }
}

function mapProps(state) {
  return { title: state.global.title };
}

export default connect(mapProps)(AppPage);
