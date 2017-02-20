//! Documentation page with documentation on the platform, the MM interface, and general help resources.

import React from 'react';
import { Row, Col } from 'antd';

import ContentContainer from '../components/ContentContainer';
import DocSearcher from '../components/docs/DocSearcher';
import DocCreator  from '../components/docs/DocCreator';
import DocViewer   from '../components/docs/DocViewer';

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
      <DocViewer />
    </div>
  );
}

export default () => { return (
  <ContentContainer title="Documentation + Help">
    <Documentation />
  </ContentContainer>
);};
