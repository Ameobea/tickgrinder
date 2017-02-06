//! A page of the application.  Contains the basic structure of the GUI including title, header, navigation, and footer.

import React from 'react';
import { connect } from 'dva';

import Header from './Header';
import styles from '../static/css/globalStyle.css';

class AppPage extends React.Component {
  render() {
    console.log(this.props);
    return (
      <div className={styles.application}>
        <Header title={this.props.title} />
        <div className={styles.content}>
          {this.props.children}
        </div>
      </div>
    );
  }
}

export default connect()(AppPage);
