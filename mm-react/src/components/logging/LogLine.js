//! A single log line entry

import { connect } from 'dva';
import { Row, Col, Tag, Tooltip } from 'antd';
const CheckableTag = Tag.CheckableTag;

import logStyles from '../../static/css/logging.css';

/// Render a pretty severity level
const Severity = connect()(({dispatch, level, onClick, closable}) => {
  let color;
  switch(level) {
    case "Debug":
      color = "cyan-inverse";
      break;
    case "Notice":
      color = "blue-inverse";
      break;
    case "Warning":
      color = "yellow-inverse";
      break;
    case "Error":
      color = "orange-inverse";
      break;
    case "Critical":
      color = "red-inverse";
      break;
  }

  return (
    <Tag
      closable={closable}
      onClick={() => onClick(dispatch)}
      color={color}
    >
      {level}
    </Tag>);
});

const Instance = ({dispatch, sender}) => {
  let instance_type = sender.instance_type;
  return (
    <Tooltip placement="right" title={sender.uuid}>
      <CheckableTag onChange={() => dispatch({type: 'logging/instanceAdded', item: sender})}>
        {instance_type}
      </CheckableTag>
    </Tooltip>
  );
}

const MessageType = connect()(({dispatch, children}) => {
  return <CheckableTag onChange={() => dispatch({type: 'logging/categoryAdded', item: children})}>{children}</CheckableTag>;
});

const LogLine = ({dispatch, msg}) => {
  return (
    <Row className={msg.level + ' ' + logStyles.logLine} type="flex" justify="space-around" align="middle">
      <Col span={2}>
        <Instance
          sender={msg.sender}
          dispatch={dispatch}
        />
      </Col>
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
