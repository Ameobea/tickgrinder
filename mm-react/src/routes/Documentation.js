//! Documentation page with documentation on the platform, the MM interface, and general help resources.
/* global CKEDITOR */

import React from 'react';
import { Row, Col } from 'antd';

import ContentContainer from '../components/ContentContainer';
import DocSearcher from '../components/docs/DocSearcher';
import DocCreator  from '../components/docs/DocCreator';
import DocViewer   from '../components/docs/DocViewer';

/**
 * Sets the contents of the editor window to that of the currently selected document and sets internal state
 * to identify that it is being edited rather than a new doc being created.
 *
 * CURRENTLY IMPOSSIBLE until Tantivy implements deletes (coming soon as of 20-02-17)
 */
const editDocument = ({dispatch}) => {
  return () => {
    // TODO
  }
};

function Documentation() {
  return (
    <div>
      <Row>
        <Col span={18}>
          <DocCreator />
        </Col>
        <Col span={6}>
          <DocSearcher />
        </Col>
      </Row>
      <DocViewer editDocument={editDocument}/>
    </div>
  );
}

export default () => { return (
  <ContentContainer title="Documentation + Help">
    <Documentation />
  </ContentContainer>
);};
