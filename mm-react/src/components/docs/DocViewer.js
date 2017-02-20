//! Renders the selected document that is returned from the document store.

import React from 'react';
import { connect } from 'dva';
var HtmlToReactParser = require('html-to-react').Parser;

import { DocumentShape } from '../../utils/commands';

const DocViewer = ({dispatch, selectedDoc}) => {
  let {title, body, tags} = selectedDoc;
  const htmlToReactParser = new HtmlToReactParser();
  const RenderedBody = htmlToReactParser.parse('<div>' + body + '</div>');

  return (
    <div className='docViewer'>
      <h3>{title}</h3>
      {RenderedBody}
    </div>
  );
};

DocViewer.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  selectedDoc: React.PropTypes.shape(DocumentShape).isRequired,
};

function mapProps(state) {
  return {
    selectedDoc: state.documents.returnedDoc,
  };
}

export default connect(mapProps)(DocViewer);
