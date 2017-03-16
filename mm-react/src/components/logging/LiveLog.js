//! A live view of log messages.

import { connect } from 'dva';
import React from 'react';
import { Row, Col, Card, Tag, Checkbox, Tooltip } from 'antd';

import styles from '../../static/css/logging.css';
import { InstanceShape } from '../../utils/commands';
import { LogLine, Severity } from './LogLine';

const handleCategoryClose = (dispatch, name) => {
  dispatch({type: 'logging/categoryClosed', item: name});
};

const handleInstanceClose = (dispatch, sender) => {
  dispatch({type: 'logging/instanceClosed', item: sender});
};

const SelectedTags = connect()(({dispatch, selected_categories, selected_severities, selected_instances, inclusive}) => {
  const handleClick = (dispatch, level) => dispatch({type: 'logging/severityClosed', item: level});
  let tags = [];
  for(var i=0; i<selected_severities.length; i++) {
    let name = selected_severities[i];
    let tag = (
      <Severity
        closable
        key={name}
        level={name}
        onClick={handleClick}
      />
    );
    tags.push(tag);
  }

  const catClickHandler = name => handleCategoryClose(dispatch, name);

  for(var j=0; j<selected_categories.length; j++) {
    let name = selected_categories[j];
    let tag = (
      <Tag closable
        color="blue"
        key={'category-' + name}
        onClick={function(){catClickHandler(name);}}
      >
        {name}
      </Tag>
    );
    tags.push(tag);
  }

  for(var k=0; k<selected_instances.length; k++) {
    let sender = selected_instances[k];
    const handleInstClick = () => handleInstanceClose(dispatch, sender);

    let tag = (
      <Tooltip
        key={'instance-' + sender.uuid}
        placement="right"
        title={sender.uuid}
      >
        <Tag
          closable
          color="green"
          onClick={handleInstClick}
        >
          {sender.instance_type}
        </Tag>
      </Tooltip>
    );
    tags.push(tag);
  }

  const handleToggle = (e) => dispatch({type: 'logging/toggleMatch'});

  return (
    <Card>
      <Checkbox
        checked={inclusive}
        onChange={handleToggle}
      >
        {'Match lines containing'}
      </Checkbox><br />
      <Checkbox
        checked={!inclusive}
        onChange={handleToggle}
      >
        {'Match lines not containing'}
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
      (((selected_instances.filter(sender => sender.uuid == log_line.cmd.Log.msg.sender.uuid).length !== 0) == inclusive) || (selected_instances.length === 0));
    if(contains) {
      rows.push(
        <LogLine
          key={log_line.uuid}
          msg={log_line.cmd.Log.msg}
        />
      );
    }
  }

  return (
    <div className={styles.liveLog}>
      <SelectedTags
        inclusive={inclusive}
        selected_categories={selected_categories}
        selected_instances={selected_instances}
        selected_severities={selected_severities}
      />
      <Row>
        <Col span={2}><b>{'Sending Instance'}</b></Col>
        <Col span={2}><b>{'Event Type'}</b></Col>
        <Col span={18}><b>{'Message'}</b></Col>
        <Col span={2}><b>{'Severity'}</b></Col>
      </Row>
      {rows}
    </div>
  );
};

LiveLog.propTypes = {
  dispatch: React.PropTypes.func.isRequired,
  inclusive: React.PropTypes.bool.isRequired,
  log_cache: React.PropTypes.arrayOf(React.PropTypes.shape({
    uuid: React.PropTypes.string,
    cmd: React.PropTypes.shape({
      Log: React.PropTypes.shape({
        msg: React.PropTypes.shape({
          level: React.PropTypes.string.isRequired,
          sender: React.PropTypes.shape(InstanceShape).isRequired,
        }).isRequired,
      }).isRequired,
    }).isRequired,
  })).isRequired,
  selected_categories: React.PropTypes.arrayOf(React.PropTypes.string).isRequired,
  selected_instances: React.PropTypes.arrayOf(React.PropTypes.shape(InstanceShape).isRequired).isRequired,
  selected_severities: React.PropTypes.arrayOf(React.PropTypes.string).isRequired
};

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
