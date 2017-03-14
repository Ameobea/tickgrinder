//! A component that's meant to be included in a form.  It contains a `Select` dropdown box that can be used to pick a data
//! downloader.  Depending on the current selection, it also renders a `TickSink` component which contains input fields
//! for the user to fill out and create a `HistTickDst`.

import React from 'react';
import { connect } from 'dva';
import { Select } from 'antd';
const Option = Select.Option;

import { tickSinkDefs, TickSink } from '../../utils/data_util';

const dstOpts = [];
for(let dstName in tickSinkDefs) {
  dstOpts.push(<Option key={dstName} value={dstName}>{tickSinkDefs[dstName].name}</Option>);
}

class DstSelector extends React.Component {
  constructor(props) {
    super(props);
    let firstSinkId = null;
    for(let dstName in tickSinkDefs) {
      firstSinkId = dstName;
      break;
    }

    this.handleSinkChange = this.handleSinkChange.bind(this);
    this.handleUpdate = this.handleUpdate.bind(this);
    this.bindRef = this.bindRef.bind(this)
    this.state = {
      sinkJsonName: firstSinkId,
    };
  }

  /**
   * Called every time the user selects a different sink from the `Select`.
   */
  handleSinkChange(sinkJsonName: string) {
    this.setState({sinkJsonName: sinkJsonName, value: this.input.getHistTickDst()});
  }

  /**
   * Called every time the sink parameters are changed by the user.  Stores the dst in the store so that it can be retrieved
   * from higher up in the scope in order to get the value in the form.
   */
  handleUpdate(dst) {
    this.props.dispatch({type: 'data/newDst', dst: dst});
  }

  bindRef(child) {
    this.input = child;
  }

  render() {
    return (
      <div>
        <Select default={dstOpts[0]} onChange={this.handleSinkChange}>
          {dstOpts}
        </Select>

        <TickSink ref={this.bindRef} sinkJsonName={this.state.sinkJsonName} handleUpdate={this.handleUpdate} />
      </div>
    );
  }
}

DstSelector.PropTypes = {
  value: React.PropTypes.string,
};

export default connect()(DstSelector);
