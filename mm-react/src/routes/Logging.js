//! Logging interface

import React from 'react';

import ContentContainer from '../components/ContentContainer';
import LiveLog from '../components/logging/LiveLog';

function LoggingPage() {
  return (
      <div>
          <LiveLog />
      </div>
  );
}

export default () => { return (
    <ContentContainer title="Log Interface">
        <LoggingPage />
    </ContentContainer>
);};
