//! Documentation page with documentation on the platform, the MM interface, and general help resources.

import React from 'react';
import { Row, Col } from 'antd';

import ContentContainer from '../components/ContentContainer';
import DocSearcher from '../components/DocSearcher';

function Documentation() {
  return (
    <div>
      <Row>
        <Col span={12}>
          {/* TODO: Note system for writing journals (with markdown editor), saving + indexing them, tagging, etc.*/}
        </Col>
        <Col span={12}>
          <DocSearcher />
        </Col>
      </Row>
    </div>
  );
}

export default () => { return (
  <ContentContainer title="Documentation + Help">
    <Documentation />
  </ContentContainer>
);};
