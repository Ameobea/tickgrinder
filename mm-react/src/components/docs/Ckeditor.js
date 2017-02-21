//! Creates a ckeditor instance.  Contains options for taking callbacks involved with saving changes.
/* global CKEDITOR */

import React from 'react';
import { connect } from 'dva';

/**
 * After the CKEditor plugin has loaded, initialize the editor
 */
function awaitCk(rand) {
  setTimeout(() => {
    let ckeditorLoaded = true;
    try{ CKEDITOR; }
    catch(e) {
      if(e.name == 'ReferenceError') {
        ckeditorLoaded = false;
      }
    }

    if(ckeditorLoaded) {
      CKEDITOR.replace( `ckeditor-${rand}` );
    } else {
      awaitCk(rand);
    }
  }, 50);
}

class CKEditor extends React.Component {
  componentDidMount() {
    // add a script tag onto the document that loads the CKEditor script
    let ckeditor_src = document.createElement('script');
    ckeditor_src.type = 'text/javascript';
    ckeditor_src.async = true;
    ckeditor_src.src='/ckeditor/ckeditor.js';
    document.getElementById('ckeditor-' + this.props.rand).appendChild(ckeditor_src);

    // wait for the CKEditor script to load and then initialize the editor
    awaitCk(this.props.rand);

    // register our id as the active editor instance
    this.props.dispatch({type: 'documents/setEditorId', id: this.props.rand});
  }

  shouldComponentUpdate(...args) {
    return false;
  }

  render() {
    return (
      <textarea id={'ckeditor-' + this.props.rand} />
    );
  }
}

CKEditor.propTypes = {
  rand: React.PropTypes.number.isRequired,
};

export default connect()(CKEditor);
