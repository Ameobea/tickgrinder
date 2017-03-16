//! A component to spawn a data downloader.

import React from 'react';
import { connect } from 'dva';
import { Select, Button, Form } from 'antd';
const FormItem = Form.Item;
const Option = Select.Option;

import { dataDownloaders } from '../../utils/data_util';

class DataDownloaderSpawner extends React.Component {
  handleSubmit = e => {
    e.preventDefault();
    this.props.form.validateFields((err, values) => {
      if (!err) {
        // get the value of the selected downloader's command from the form
        let cmd = this.props.form.getFieldValue('downloaderName');
        // send the command to the spawner to be executed
        this.props.dispatch({
          cmd: cmd,
          cb_action: 'instances/instanceSpawnCallback',
          instance_name: 'Spawner',
          type: 'platform_communication/sendCommandToInstance',
        });
      }
    });
  }

  render() {
    // creation `Option`s for each of the available data downloaders
    let availableDownloaders = [];
    for(var i=0; i<this.props.dataDownloaders.length; i++) {
      availableDownloaders.push(
        <Option
          key={this.props.dataDownloaders[i].name}
          value={this.props.dataDownloaders[i].command}
        >
          {this.props.dataDownloaders[i].name}
        </Option>
      );
    }

    const { getFieldDecorator } = this.props.form;
    return (
      <Form layout='inline' onSubmit={this.handleSubmit}>
        <FormItem>
          {getFieldDecorator('downloaderName', {
            rules: [
              { required: true, message: 'Please select a downloader to spawn!' },
            ],
          })(
            <Select placeholder='Select a downloader to spawn' style={{width: 250}}>
              {availableDownloaders}
            </Select>
          )}
        </FormItem>

        <FormItem>
          {getFieldDecorator('button', {})(
            <Button htmlType='submit' type='primary'>
              {'Spawn'}
            </Button>
          )}
        </FormItem>
      </Form>
    );
  }
}

DataDownloaderSpawner.propTypes = {
  dataDownloaders: React.PropTypes.arrayOf(React.PropTypes.shape({
    name: React.PropTypes.string.isRequired,
    command: React.PropTypes.string.isRequired,
    description: React.PropTypes.string.isRequired,
  })).isRequired,
  dispatch: React.PropTypes.func.isRequired,
  form: React.PropTypes.any.isRequired,
};

function mapProps(state) {
  return {
    dataDownloaders: dataDownloaders,
  };
}

export default connect(mapProps)(Form.create()(DataDownloaderSpawner));
