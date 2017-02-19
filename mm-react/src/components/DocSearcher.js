//! Component used to search through the stored documents, documentation, and other data that comes along with the platform or
//! are created by the user via the integrated journaling/logging tools.

import React from 'react';
import { connect } from 'dva';
import { AutoComplete, Input } from 'antd';

const returnTrue = () => true;

class DocSearcher extends React.Component {
  componentWillMount() {
    // register outself as a recipient of query results and re-draw ourself when they're received
    this.props.dispatch({type: 'platform_communication/registerDocQueryReceiver', cb: (results) => {
      // simulate a click on the input box to force the component to update
      document.getElementById('autocompleteInput').click();
    }});
  }

  handleDocInputSelect(dispatch) {
    return (value, option) => {
      console.log(value);
      console.log(option);
      // TODO
    };
  }

  handleDocInputChange(dispatch) {
    return (value, label) => {
      dispatch({type: 'platform_communication/sendDocQuery', query: value});
    };
  }

  render() {
    return (
      <div>
        <h2>{'Search Documentation'}</h2>
        <AutoComplete
          dataSource={this.props.queryResults}
          filterOption={returnTrue}
          onChange={this.handleDocInputChange(this.props.dispatch)}
          onSelect={this.handleDocInputSelect(this.props.dispatch)}
          placeholder="Enter a term to search for in the documentation"
          style={{ width: 200 }}
        >
          <Input id="autocompleteInput" />
        </AutoComplete>
      </div>
    );
  }
}

DocSearcher.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  queryResults: React.PropTypes.arrayOf(React.PropTypes.string).isRequired,
};

function mapProps(state) {
  return {
    queryResults: state.platform_communication.queryResults,
  };
}

export default connect(mapProps)(DocSearcher);
