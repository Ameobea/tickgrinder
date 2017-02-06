//! A wrapper for the main content that is unique to the currently selected page.

import React from 'react';

function wrapContent(title, content) {
  return function({dispatch}) {
    dispatch({type: 'global/pageChange', title: title});
    return content();
  }
}

export default wrapContent;
