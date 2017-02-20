//! Renders the selected document that is returned from the document store.

import React from 'react';
import { connect } from 'dva';
import { Button } from 'antd';
var HtmlToReactParser = require('html-to-react').Parser;

import { DocumentShape } from '../../utils/commands';
import gstyles from '../../static/css/globalStyle.css';

const DocViewer = ({dispatch, selectedDoc, editDocument}) => {
  let {title, body, tags} = selectedDoc;
  const htmlToReactParser = new HtmlToReactParser();
  const RenderedBody = htmlToReactParser.parse('<div>' + body + '</div>');

  return (
    <div className='docViewer' className={gstyles.leftText}>
      <br />
      <h1 className={gstyles.inlineH1} >{title}</h1>
      <Button disabled onClick={editDocument(dispatch)} type='primary'>Edit</Button>
      <Button type='danger' disabled>Delete</Button>
      <hr />
      {RenderedBody}
    </div>
  );
};

DocViewer.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  selectedDoc: React.PropTypes.shape(DocumentShape).isRequired,
  editDocument: React.PropTypes.func.isRequired,
};

function mapProps(state) {
  return {
    selectedDoc: state.documents.returnedDoc,
  };
}

export default connect(mapProps)(DocViewer);
