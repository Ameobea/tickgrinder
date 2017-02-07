//! Logging interface

import React from 'react';

import ContentContainer from '../components/ContentContainer';

function LoggingPage() {
	return (
		<div>logging</div>
	);
}

export default () => <ContentContainer title="Log Interface" content_function={LoggingPage} />;
