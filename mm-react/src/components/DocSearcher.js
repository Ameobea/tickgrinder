//! Component used to search through the stored documents, documentation, and other data that comes along with the platform or
//! are created by the user via the integrated journaling/logging tools.

import React from 'react';
import { connect } from 'dva';
import { AutoComplete } from 'antd';

/**
 * Returns a function that submits a query to the document store which will eventually update the state's `queryResults`
 * attribute when it completes.
 */
const handleDocInputChange = dispatch => {
  return (value) => {
    dispatch({type: 'platform_communication/sendDocQuery', query: value});
  };
};

/**
 * Returns a function that, given the doc page selected by the user, brings that doc up in the main view.
 */
const handleDocInputSelect = dispatch => {
  return (selected) => {
    // TODO
  };
};

const DocSearcher = ({dispatch, queryResults}) => {
  console.log(queryResults);
  return (
    <div>
      <h2>{'Search Documentation'}</h2>
      <AutoComplete
        dataSource={queryResults}
        filterOption={() => true}
        onChange={handleDocInputChange(dispatch)}
        onSelect={handleDocInputSelect(dispatch)}
        placeholder="Enter a term to search for in the documentation"
        style={{ width: 200 }}
      />
    </div>
  );
};

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
