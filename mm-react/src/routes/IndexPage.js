//! Front page of the dashboard

import React from 'react';

import ContentContainer from '../components/ContentContainer';

function IndexPage() {
  return (
    <div>Index</div>
  );
}

export default () => <ContentContainer title="Dashboard Homepage" content_function={IndexPage} />;
