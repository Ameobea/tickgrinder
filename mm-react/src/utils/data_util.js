//! Declares platform constants and metadata used to send commands.
// @flow

import React from 'react';
import { Input } from 'antd';

type DataDownloaderDefinition = {name: string, description: string, command: string, supportedDownloaders: Array<string>};

/**
 * Contains a list of all data downloaders and information about them including their description, name, and
 * the command used to spawn them.
 */
const dataDownloaders: Array<DataDownloaderDefinition> = [
  {
    name: 'FXCM Native Data Downloader',
    description: 'Uses the native FXCM C++ API to request historical ticks programatically.  Requires ' +
      'valid FXCM demo account credentials.',
    command: 'SpawnFxcmNativeDataDownloader',
    supportedDownloaders: ['Flatfile', 'Postgres', 'RedisChannel', 'RedisSet', 'Console']
  }, {
    name: 'FXCM Flatfile Data Downloader',
    description: 'Retrieves the FXCM-hosted flatfile archives containing historical tick data and processes ' +
      'them into CSV files.',
    command: 'SpawnFxcmFlatfileDataDownloader',
    supportedDownloaders: ['Flatfile', 'Console']
  }, {
    name: 'IEX Data Downloader',
    description: 'Uses the live data streams from the Investor\'s Exchange (IEX) to get live price data for US stocks.',
    command: 'SpawnIexDataDownloader',
    supportedDownloaders: ['Flatfile', 'Console']
  }, {
    name: 'Poloniex Data Downloader',
    description: 'Hooks into the Poloniex API to retrieve live streaming orderbook and trade updates as well as ' +
      'historical trade data.  Only supports writing to Flatfile.',
    command: 'SpawnPoloniexDataDownloader',
    supportedDownloaders: ['Flatfile', 'Console']
  }
];

/**
 * `TickSink`s are react components that contain an extra function that, using the state of the component, creates a `HistTickDst`
 * that can be used along with a backtest.
 *
 * Use like `<TickSink sinkJsonName='Flatfile' handleUpdate={this.handleUpdate} />`
 *
 * These components also compose a `getHistTickDst()` method that returns a JSON-encoded `HistTickDst` or null if the user neglected
 * to input one or more of the required input fields.
 */
class TickSink extends React.Component {
  constructor(props: {sinkJsonName: string}) {
    super(props);
    this.handleParamChange = this.handleParamChange.bind(this);
    this.state = this.getDef(props.sinkJsonName);
  }

  componentWillReceiveProps(nextProps) {
    if(nextProps.sinkJsonName != this.props.sinkJsonName) {
      this.setState(this.getDef(nextProps.sinkJsonName));
    }
  }

  getDef(sinkJsonName) {
    // create `TickSink`s out of all of the supplied sink definitions
    let sinkDef = tickSinkDefs[sinkJsonName];
    let paramDefs = sinkDef.params;
    const sinkParams = paramDefs.map(
      (paramDef: {clearName: string, jsonName: string, paramType: string}): TickSinkParam<any> => {
        paramDef.changeHandler = this.handleParamChange;
        return tickSinkParamGens[paramDef.paramType](paramDef);
      }
    );

    return {
      sinkParams: sinkParams,
      // hold the values from the parameter inputs
      sinkParamVals: {},
      sinkDef: sinkDef,
    };
  }

  handleParamChange(name, e) {
    let paramVals = this.state.sinkParamVals;
    paramVals[name] = e.target.value;
    this.setState({sinkParamVals: paramVals});

    // generate the `HistTickDst` from the parameter values of this object and submit it to the handler
    let dst = this.getHistTickDst();
    this.props.handleUpdate(dst);
  }

  /**
   * Retrieves the values of all of the sink's parameters, collectes them into an object, and returns it as a JSON-encoded string.
   * This should be able to be included in a `DownloadTicks` or similar command.  If any of the parameters are missing values,
   * this function returns `null`.
   */
  getHistTickDst() {
    let dst = {};

    // for sinks that have no params, the command is just a string
    if(this.state.sinkDef.params.length === 0) {
      return JSON.stringify(this.props.sinkJsonName);
    }

    // merge the values of all the parameters into one object
    for(var i=0; i<this.state.sinkDef.params.length; i++) {
      let key = this.state.sinkDef.params[i].jsonName;
      let val = this.state.sinkParamVals[key]
      // return an early null if any of the parameters are null (due to the user not filling them out)
      if(val === null || val === undefined) {
        return null;
      }

      dst[key] = val;
    }

    // convert the dst to format `{SinkName: {...}}` and return as a JSON-encoded string
    let finalDestination = {}; // ;)
    finalDestination[this.props.sinkJsonName] = dst;
    return JSON.stringify(finalDestination);
  }

  render() {
    return (
      <div>
        <h2>{this.state.sinkDef.name}</h2>
        {this.state.sinkParams}
      </div>
    );
  }
}

TickSink.PropTypes = {
  sinkJsonName: React.PropTypes.string.isRequired,
  handleUpdate: React.PropTypes.func.isRequired,
};

/**
 * A React component that also exposes a `getValue()` function to retrieve the user-supplied sink parameter (or null if the user faile to supply a value)
 */
type TickSinkParam<T> = React.Component & {getValue: () => ?T};

/**
 * A function that, given the name of the parameter, returns a `TickSinkParam` for it.
 */
type TickSinkParamGenerator<T> = (clearName: string, jsonName: string, changeHandler: (paramName: string, paramValue: ?string) => void) => TickSinkParam<T>;

/**
 * A sink paramater that creates a simple text input box and returns the value entered into it.
 */
class StringInput extends React.Component {
  constructor(props) {
    super(props);
    this.bindRef = this.bindRef.bind(this);
    this.handleChange = this.handleChange.bind(this);
  }

  bindRef(child) {
    this.input = child;
  }

  getValue(): ?string {
    if(this.input.value == '') {
      return null;
    }

    return JSON.parse(`{${this.props.jsonName}: ${this.input.value}}`);
  }

  handleChange(val) {
    this.props.changeHandler(this.props.jsonName, val);
  }

  render() {
    return <Input onChange={this.handleChange} placeholder={this.props.clearName} ref={this.bindRef} />;
  }
}

StringInput.propTypes = {
  changeHandler: React.PropTypes.func,
  clearName: React.PropTypes.string,
  jsonName: React.PropTypes.string,
};

/**
 * All parameter generator functions available for tick sinks to use in their components
 */
const tickSinkParamGens: { [key: string]: TickSinkParamGenerator<any> } = {
  str: ({clearName, jsonName, changeHandler}) => {
    return (
      <StringInput
        changeHandler={changeHandler}
        clearName={clearName}
        jsonName={jsonName}
        key={clearName}
      />
    );
  }
};

/**
 * A set of definitions for props that represent all possible places that ticks can be sent to.  They contain the names of all paramaters that are required
 * in order to build the `HistTickDst`s internally.
 */
const tickSinkDefs
  : { [key: string]: {name: string, params: Array<{clearName: string, jsonName: string, paramType: TickSinkParam}>} }
 = {
   Flatfile: {name: 'Flatfile', params: [{clearName: 'Filename', jsonName: 'filename', paramType: 'str'}]},
   Postgres: {name: 'PostgreSQL Table', params: [
    {clearName: 'Table', jsonName: 'table', paramType: 'str'}
   ]},
   RedisChannel: {name: 'Redis Channel', params: [
    {clearName: 'Redis Host', jsonName: 'host', paramType: 'str'},
    {clearName: 'Channel', jsonName: 'channel', paramType: 'str'},
   ]},
   RedisSet: {name: 'Redis Set', params: [
    {clearName: 'Redis Host', jsonName: 'host', paramType: 'str'},
    {clearName: 'Set Name', jsonName: 'set_name', paramType: 'str'},
   ]},
   Console: {name: 'Stdout', params: []},
 };

export default {
  dataDownloaders: dataDownloaders,
  TickSink: TickSink,
  tickSinkDefs: tickSinkDefs,
};
