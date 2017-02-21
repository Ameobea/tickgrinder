//! Data Management and Collection Interface

import React from 'react';

import ContentContainer from '../components/ContentContainer';
import DataDownloader from '../components/data/DataDownloader';
import DataManager from '../components/data/DataManager';

function DataManagement() {
  return (
    <div>
      <h1>Data Downloader</h1>
      <DataDownloader />
      <h1>Data Manager</h1>
      <DataManager />
    </div>
  );
}

export default () => { return (
  <ContentContainer title="Data Management">
    <DataManagement />
  </ContentContainer>
);};
