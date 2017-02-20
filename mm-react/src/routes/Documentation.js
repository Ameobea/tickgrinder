//! Documentation page with documentation on the platform, the MM interface, and general help resources.

import React from 'react';
import { Row, Col } from 'antd';

import ContentContainer from '../components/ContentContainer';
import DocSearcher from '../components/DocSearcher';
import DocCreator  from '../components/DocCreator';

function Documentation() {
  return (
    <div>
      <Row>
        <Col span={12}>
          <DocCreator />
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
