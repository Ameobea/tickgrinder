//! Documentation page with documentation on the platform, the MM interface, and general help resources.

import React from 'react';

import ContentContainer from '../components/ContentContainer';

function Documentation() {
  return (
    <div>
      {'Help'}
      {/* TODO: fuzzy search for documentation and user-defined documents, logs, etc. */}
      {/* TODO: Note system for writing journals (with markdown editor), saving + indexing them, tagging, etc.*/}
    </div>
  );
}

export default () => { return (
  <ContentContainer title="Documentation + Help">
    <Documentation />
  </ContentContainer>
);};
