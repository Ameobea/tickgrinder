//! A single log line entry

import { connect } from 'dva';
import React from 'react';
import { Row, Col, Tag, Tooltip } from 'antd';
const CheckableTag = Tag.CheckableTag;

import logStyles from '../../static/css/logging.css';

/// Render a pretty severity level
const Severity = connect()(({dispatch, level, onClick, closable}) => {
  let color;
  switch(level) {
  case 'Debug':
    color = 'cyan-inverse';
    break;
  case 'Notice':
    color = 'blue-inverse';
    break;
  case 'Warning':
    color = 'yellow-inverse';
    break;
  case 'Error':
    color = 'orange-inverse';
    break;
  case 'Critical':
    color = 'red-inverse';
    break;
  }

  const handleClick = () => {
    onClick(dispatch);
  };

  return (
      <Tag
          closable={closable}
          color={color}
          onClick={handleClick}
      >
          {level}
      </Tag>);
});

const Instance = ({dispatch, sender}) => {
  let instance_type = sender.instance_type;
  const handleChange = () => dispatch({type: 'logging/instanceAdded', item: sender});

  return (
      <Tooltip
          placement="right"
          title={sender.uuid}
      >
          <CheckableTag onChange={handleChange}>
              {instance_type}
          </CheckableTag>
      </Tooltip>
  );
};

Instance.propTypes = {
  dispatch: React.PropTypes.function.isRequired,
  sender: React.PropTypes.object.isRequired,
};

const MessageType = connect()(({dispatch, children}) => {
  const handleChange = () => dispatch({type: 'logging/categoryAdded', item: children});
  return <CheckableTag onChange={handleChange}>{children}</CheckableTag>;
});

const LogLine = ({dispatch, msg}) => {
  const handleClick = dispatch => dispatch({type: 'logging/severityAdded', item: msg.level});

  return (
      <Row
          align="middle"
          className={msg.level + ' ' + logStyles.logLine}
          justify="space-around"
          type="flex"
      >
          <Col span={2}>
              <Instance
                  dispatch={dispatch}
                  sender={msg.sender}
              />
          </Col>
          <Col span={2}><MessageType>{msg.message_type}</MessageType></Col>
          <Col span={18}><div>{msg.message}</div></Col>
          <Col span={2}>
              <Severity
                  level={msg.level}
                  onClick={handleClick}
              />
          </Col>
      </Row>
  );
};

LogLine.propTypes = {
  dispatch: React.PropTypes.function.isRequired,
  msg: React.PropTypes.object.isRequired,
};

export default {
  LogLine: connect()(LogLine),
  Severity: Severity,
};
