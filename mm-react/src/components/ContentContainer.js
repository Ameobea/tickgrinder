//! A wrapper for the main content that is unique to the currently selected page.

import React, { Component, PropTypes } from 'react'
import { connect } from 'dva';

import gstyles from '../static/css/globalStyle.css';

class ContentContainer extends Component {
  static propTypes = {
    title: PropTypes.string.isRequired,
  }

  render() {
    return (
      <div className={gstyles.content}>
        { this.props.children }
      </div>
    );
  }

  componentWillMount() {
    this.props.dispatch({type: 'global/pageChange', title: this.props.title});
  }
}

export default connect()(ContentContainer);
