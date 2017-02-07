//! Logging interface

import React from 'react';
import { connect } from 'dva';

import wrapContent from '../components/ContentContainer';
import styles from '../static/css/IndexPage.css';

function LoggingPage() {
	return (
		<div className={styles.content}>
			logging
		</div>
	);
}

export default connect()(wrapContent("Log Interface", LoggingPage));
