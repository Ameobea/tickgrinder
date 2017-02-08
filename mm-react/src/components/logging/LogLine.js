//! A single log line entry

import { Row, Col } from 'antd';

const Instance = ({sender}) => {
  let instance_type = sender.instance_type;
  return (
    <div>{instance_type}</div>
  );
}

const LogLine = ({msg}) => {
  return (
    <Row>
      <Col span={2}><div><Instance sender={msg.sender} /></div></Col>
      <Col span={2}><div>{msg.message_type}</div></Col>
      <Col span={18}><div>{msg.message}</div></Col>
      <Col span={2}><div>{msg.level}</div></Col>
    </Row>
  );
}

export default LogLine;
