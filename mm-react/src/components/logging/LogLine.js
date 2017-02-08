//! A single log line entry

import { Row, Col, Tag } from 'antd';

import logStyles from '../../static/css/logging.css';

/// Render a pretty severity level
const Severity = ({level}) => {
  switch(level) {
    case "Debug":
      return <Tag color="cyan-inverse">Debug</Tag>;
      break;
    case "Notice":
      return <Tag color="blue-inverse">Notice</Tag>;
      break;
    case "Warning":
      return <Tag color="yellow-inverse">Warning</Tag>;
      break;
    case "Error":
      return <Tag color="orange-inverse">Error</Tag>;
      break;
    case "Critical":
      return <Tag color="red-inverse">Critical</Tag>;
      break;
  }
}

const Instance = ({sender}) => {
  let instance_type = sender.instance_type;
  return (
    <div>{instance_type}</div>
  );
}

const LogLine = ({msg}) => {
  return (
    <Row className={msg.level + ' ' + logStyles.logLine} type="flex" justify="space-around" align="middle">
      <Col span={2}><div><Instance sender={msg.sender} /></div></Col>
      <Col span={2}><div>{msg.message_type}</div></Col>
      <Col span={18}><div>{msg.message}</div></Col>
      <Col span={2}><div><Severity level={msg.level} /></div></Col>
    </Row>
  );
}

export default LogLine;
