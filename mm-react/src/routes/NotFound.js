//! Default display route for 404s

import React from 'react';

import ContentContainer from '../components/ContentContainer';

function NotFound() {
  return (
    <div>
      <h1>Not Found</h1>
      <p>This page doesn't exist; looks like you entered a wrong link or something.<br />
      If you got here by clicking a link, please submit an issue about it <a href="https://github.com/Ameobea/tickgrinder/issues">here</a>!</p>
    </div>
  );
}

export default () => { return (
  <ContentContainer title="Not Found">
    <NotFound />
  </ContentContainer>
);};
