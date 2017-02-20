//! Creates a component with a CKEditor built in that allows users to create new documents and save them to the store.
/* global CKEDITOR */

import React from 'react';
import { connect } from 'dva';
import { Input, Button } from 'antd';

import CKEditor from './Ckeditor';
import gstyles from '../../static/css/globalStyle.css';

/**
 * Returns a function that gets the HTML content from the inner CKEditor instance and saves it to the document store
 */
const saveDocument = (dispatch, rand) => {
  return () => {
    let content = CKEDITOR.instances['ckeditor-' + rand].getData();
    dispatch({
      type: 'documents/saveDocument',
      title: document.getElementById('ck-title-' + rand).value,
      tags: document.getElementById('ck-tags-' + rand).value.split(" "),
      body: content,
    });
  };
};

const DocCreator = ({dispatch}) => {
  // random identifier for this editor, because there may be multiple on one page
  let rand = Math.floor(Math.random() * (1000000 + 1));

  return (
    <div className={gstyles.leftText}>
      <CKEditor rand={rand} />
      <br />
      {'Enter a title for this document'}
      <Input id={'ck-title-' + rand} />
      {'Enter space-separated tags for this document'}
      <Input id={'ck-tags-' + rand} />
      <Button onClick={saveDocument(dispatch, rand)} type="primary">{'Save Document'}</Button>
    </div>
  );
};

DocCreator.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
};

export default connect()(DocCreator);
