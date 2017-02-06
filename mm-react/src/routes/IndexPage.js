import React from 'react';
import { connect } from 'dva';
import styles from './IndexPage.css';
import { Switch } from 'antd';

function IndexPage() {
  return (
    <div className={styles.normal}>
      <Switch />
    </div>
  );
}

IndexPage.propTypes = {
};

export default connect()(IndexPage);
