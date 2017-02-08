//! A live view of log messages.

import { connect } from 'dva';
import { Row, Col } from 'antd';

import styles from '../../static/css/logging.css';
import LogLine from './LogLine';

const LiveLog = ({log_cache}) => {
  console.log(log_cache);
  let rows = [];
  for(let i = log_cache.length - 1; i > (log_cache.length - 26) && i >= 0; i--) {
    var log_line = log_cache[i];
    rows.push(<LogLine key={log_line.uuid} msg={log_line.cmd.Log.msg} />);
  }

  return (
    <div className={styles.liveLog}>
      <Row>
        <Col span={2}><b>Sending Instance</b></Col>
        <Col span={2}><b>Event Type</b></Col>
        <Col span={18}><b>Message</b></Col>
        <Col span={2}><b>Severity</b></Col>
      </Row>
      {rows}
    </div>
  );
}

function mapProps(state) {
  return { log_cache: state.platform_communication.log_messages };
}

export default connect(mapProps)(LiveLog);
