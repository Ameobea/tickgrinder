//! Data Management and Collection Interface

import React from 'react';

import ContentContainer from '../components/ContentContainer';
import DataDownloader from '../components/data/DataDownloader';
import DataManager from '../components/data/DataManager';
import DataDownloaderSpawner from '../components/data/DataDownloaderSpawner';

function DataManagement() {
  return (
    <div>
      <h1>Spawn Data Downloader</h1>
      <DataDownloaderSpawner />
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
