//! A live view of log messages.

import { connect } from 'dva';
import { Row, Col, AutoComplete, Card, Tag, Checkbox, Tooltip } from 'antd';

import styles from '../../static/css/logging.css';
import { LogLine, Severity, Instance } from './LogLine';

const handleSeverityClose = (dispatch, name) => {
  dispatch({type: 'logging/severityRemoved', item: name});
}

const handleCategoryClose = (dispatch, name) => {
  dispatch({type: 'logging/categoryClosed', item: name});
}

const handleInstanceClose = (dispatch, sender) => {
  dispatch({type: 'logging/instanceClosed', item: sender});
}

const SelectedTags = connect()(({dispatch, selected_categories, selected_severities, selected_instances, inclusive}) => {
  let tags = [];
  for(var i=0; i<selected_severities.length; i++) {
    let name = selected_severities[i];
    let tag = (
      <Severity
        key={name}
        closable
        level={name}
        onClick={dispatch => dispatch({type: 'logging/severityClosed', item: name})}
      />
    );
    tags.push(tag);
  }

  for(var i=0; i<selected_categories.length; i++) {
    let name = selected_categories[i];
    let tag = <Tag closable color="blue" onClick={() => handleCategoryClose(dispatch, name)} key={'category-' + name}>{name}</Tag>;
    tags.push(tag);
  }

  for(var i=0; i<selected_instances.length; i++) {
    let sender = selected_instances[i];
    let tag = (
      <Tooltip placement="right" title={sender.uuid} key={'instance-' + sender.uuid}>
        <Tag closable color="green" onClick={() => handleInstanceClose(dispatch, sender)}>
          {sender.instance_type}
        </Tag>
      </Tooltip>
    );
    tags.push(tag);
  }

  return (
    <Card>
      <Checkbox onChange={(e) => dispatch({type: 'logging/toggleMatch'})} checked={inclusive}>
        Match lines containing
      </Checkbox><br/>
      <Checkbox onChange={(e) => dispatch({type: 'logging/toggleMatch'})} checked={!inclusive}>
        Match lines not containing
      </Checkbox>
      {tags}
    </Card>
  );
});

const LiveLog = ({dispatch, log_cache, selected_categories, selected_severities, selected_instances, inclusive}) => {
  let rows = [];
  for(let i = log_cache.length - 1; i > (log_cache.length - 26) && i >= 0; i--) {
    var log_line = log_cache[i];
    // check if the log line should be displayed based on the selected categories/severities and inclusiveness state
    let contains = ((selected_severities.indexOf(log_line.cmd.Log.msg.level) != -1 == inclusive) || (selected_severities.length === 0)) &&
      ((selected_categories.indexOf(log_line.cmd.Log.msg.message_type) != -1 == inclusive) || (selected_categories.length === 0)) &&
      (((selected_instances.filter(sender => sender.uuid != log_line.cmd.Log.msg.sender.uuid).length == 0) == inclusive) || (selected_instances.length === 0));
    if(contains) {
      rows.push(<LogLine key={log_line.uuid} msg={log_line.cmd.Log.msg} />);
    }
  }

  return (
    <div className={styles.liveLog}>
      <SelectedTags
        selected_severities={selected_severities}
        selected_categories={selected_categories}
        selected_instances={selected_instances}
        inclusive={inclusive}
      />
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
  return {
    log_cache: state.platform_communication.log_messages,
    selected_categories: state.logging.selected_categories,
    selected_severities: state.logging.selected_severities,
    selected_instances:  state.logging.selected_instances,
    inclusive: state.logging.inclusive,
  };
}

export default connect(mapProps)(LiveLog);
