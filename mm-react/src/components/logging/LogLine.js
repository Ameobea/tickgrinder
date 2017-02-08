//! A single log line entry

import { connect } from 'dva';
import { Row, Col, Tag } from 'antd';
const CheckableTag = Tag.CheckableTag;

import logStyles from '../../static/css/logging.css';

/// Render a pretty severity level
const Severity = connect()(({dispatch, level, onClick, closable}) => {
  switch(level) {
    case "Debug":
      return <Tag closable={closable} onClick={() => onClick(dispatch)} color="cyan-inverse">Debug</Tag>;
      break;
    case "Notice":
      return <Tag closable={closable} onClick={() => onClick(dispatch)} color="blue-inverse">Notice</Tag>;
      break;
    case "Warning":
      return <Tag closable={closable} onClick={() => onClick(dispatch)} color="yellow-inverse">Warning</Tag>;
      break;
    case "Error":
      return <Tag closable={closable} onClick={() => onClick(dispatch)} color="orange-inverse">Error</Tag>;
      break;
    case "Critical":
      return <Tag closable={closable} onClick={() => onClick(dispatch)} color="red-inverse">Critical</Tag>;
      break;
  }
});

const Instance = ({sender}) => {
  let instance_type = sender.instance_type;
  return (
    <div>{instance_type}</div>
  );
}

const MessageType = connect()(({dispatch, children}) => {
  return <CheckableTag onChange={() => dispatch({type: 'logging/categoryAdded', item: children})}>{children}</CheckableTag>;
});

const LogLine = ({dispatch, msg}) => {
  return (
    <Row className={msg.level + ' ' + logStyles.logLine} type="flex" justify="space-around" align="middle">
      <Col span={2}><Instance sender={msg.sender} /></Col>
      <Col span={2}><MessageType>{msg.message_type}</MessageType></Col>
      <Col span={18}><div>{msg.message}</div></Col>
      <Col span={2}>
        <Severity
          level={msg.level}
          onClick={dispatch => dispatch({type: 'logging/severityAdded', item: msg.level})}
        />
      </Col>
    </Row>
  );
}

export default {
  LogLine: connect()(LogLine),
  Severity: Severity,
}
