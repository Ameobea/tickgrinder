//! Front page of the dashboard

import React from 'react';

import ContentContainer from '../components/ContentContainer';

function IndexPage() {
  return (
    <div>{'Index'}</div>
  );
}

export default () => { return (
  <ContentContainer title='Dashboard Homepage'>
    <IndexPage />
  </ContentContainer>
);};

