//! Tool used to facilitate the downloading and saving of historical data or data from other sources
//! in general.  Will show on the `Data` page of the MM.

import React from 'react';
import { connect } from 'dva';
import { Select, Button, Form, Input, DatePicker, message } from 'antd';
const Option = Select.Option;
const FormItem = Form.Item;
const { RangePicker } = DatePicker;

import { InstanceShape } from '../../utils/commands';
import DstSelector from './DstSelector';
import RunningDownloads from './RunningDownloads';

/**
 * Given a list of running instances, returns the set of those instances that are data downloaders as `Option`s
 */
const getDownloaders = livingInstances => {
  // It is assumed that all data downloader instances will have the phrase "Data Downloader" in their instance type
  return livingInstances.filter(inst => inst.instance_type.toLowerCase().indexOf('data downloader') !== -1)
    .map(inst => (
      <Option key={inst.uuid} value={inst.uuid}>
        {inst.instance_type}
      </Option>
    ));
};

// TODO: Auto-update progress of all running data downloads every few seconds

class DataDownloader extends React.Component {
  /**
   * Returns a function that, given a `HistTickDst` as an argument, attempts to start the data download.  Shows a message on error.
   */
  handleSubmit = dst => {
    return e => {
      e.preventDefault();
      this.props.form.validateFields((err, values) => {
        if (!err) {
          // get the data out of the form
          let {downloader, timeframe, symbol} = this.props.form.getFieldsValue(
            ['downloader', 'timeframe', 'symbol']
          );

          console.log(dst);
          if(dst === null) {
            message.error('Invalid input supplied for the download\'s tick sink!');
          }

          // convert start and end times to nanoseconds and send command to start the download
          let startTime = timeframe[0]._d.getTime() * 1000 * 1000;
          let endTime = timeframe[1]._d.getTime() * 1000 * 1000;
          let cmd = {DownloadTicks: {
            start_time: startTime,
            end_time: endTime,
            symbol: symbol,
            dst: JSON.parse(dst),
          }};

          // send the command to the chosen data downloader
          this.props.dispatch({
            instance_name: 'Spawner',
            cb_action: 'data/downloadRequestResponseReceived',
            channel: downloader,
            cmd: cmd,
            type: 'platform_communication/sendCommand',
          });
        }
      });
    };
  }

  render() {
    // get a list of all available data downloaders from the platform
    let available_downloaders = getDownloaders(this.props.livingInstances); // TODO

    const { getFieldDecorator } = this.props.form;
    return (
      <div>
        <h2>{'Start Data Download'}</h2>
        <Form layout='inline' onSubmit={this.handleSubmit(this.props.dst)}>
          <FormItem label='Data Downloader'>
            {getFieldDecorator('downloader', {
              rules: [{ required: true, message: 'Please select a data downloader to use for this download.'}]
            })(
              <Select style={{ width: 200 }}>
                {available_downloaders}
              </Select>
            )}
          </FormItem>

          <FormItem label='Symbol'>
            {getFieldDecorator('symbol', {
              rules: [{ required: true, message: 'Please select a symbol to download.' }],
            })(
              <Input type='text' />
            )}
          </FormItem>

          <FormItem label='Timeframe (UTC)'>
            {getFieldDecorator('timeframe', {
              rules: [{ type: 'array', required: true, message: 'Please select a start and end time.' }],
            })(
              <RangePicker
                format='YYYY-MM-DD HH:mm:ss'
                placeholder={['Start Time', 'End Time']}
                showTime
              />
            )}
          </FormItem>

          <FormItem>
            <DstSelector />
          </FormItem>

          <FormItem>
            {getFieldDecorator('button', {})(
              <Button htmlType='submit' type='primary'>
                {'Start Data Download'}
              </Button>
            )}
          </FormItem>
        </Form>

        <h2>{'Running Downloads'}</h2>
        {'<RunningDownloads />'}'
      </div>
    );
  }
}

DataDownloader.propTypes = {
  dispatch: React.PropTypes.func.isRequired,

  dst: React.PropTypes.any,
  form: React.PropTypes.any.isRequired,
  livingInstances: React.PropTypes.arrayOf(React.PropTypes.shape(InstanceShape)).isRequired,
};

function mapProps(state) {
  return {
    livingInstances: state.instances.living_instances,
    dst: state.data.dst,
  };
}

export default connect(mapProps)(Form.create()(DataDownloader));
