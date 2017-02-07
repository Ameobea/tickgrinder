//! Data Management and Collection Interface

import React from 'react';

import ContentContainer from '../components/ContentContainer';

function DataManagement() {
  return (
    <div>Data</div>
  );
}

export default () => <ContentContainer title="Data Management" content_function={DataManagement} />;
