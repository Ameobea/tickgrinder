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
 * Use like `<TickSink name='Example Sink' def={tickSinkDefs.Flatfile} />`
 *
 * These components also compose a `getHistTickDst()` method that returns a JSON-encoded `HistTickDst` or null if the user neglected
 * to input one or more of the required input fields.
 */
class TickSink extends React.Component {
  constructor(props: {name: string, jsonName: string, defs: Array<{paramName: string, paramType: string}>}) {
    super(props);
    // create `TickSink`s out of all of the supplied sink definitions
    let defs = props.defs;
    const sinks = defs.map((def: {paramName: string, paramType: string}): TickSinkParam<any> => tickSinkParamGens[def.paramType](def.paramName));

    this.state = {
      sinks: sinks,
    };
  }

  /**
   * Retrieves the values of all of the sink's parameters, collectes them into an object, and returns it as a JSON-encoded string.
   * This should be able to be included in a `DownloadTicks` or similar command.  If any of the parameters are missing values,
   * this function returns `null`.
   */
  getHistTickDst() {
    let dst = {};
    // merge the props of all the parameters into one object
    for(var i=0; i<this.state.sinks.length; i++) {
      dst = Object.assign(dst, this.state.sinks[i]);
    }

    // return an early null if any of the parameters are null (due to the user not filling them out)
    for(var key in dst) {
      if(dst[key] === null) {
        return null;
      }
    }

    // convert the dst to format `{SinkName: {...}}` and return as a JSON-encoded string
    let finalDestination = {}; // ;)
    finalDestination[this.props.jsonName] = dst;
    return JSON.stringify(finalDestination);
  }

  render() {
    return (
      <div>
        <h2>{this.props.name}</h2>
        {this.state.sinks}
      </div>
    );
  }
}

TickSink.PropTypes = {
  name: React.PropTypes.string,
  defs: React.PropTypes.arrayOf(
    React.PropTypes.shape({
      paramName: React.PropTypes.string,
      paramType: React.PropTypes.string,
    })
  ),
};

/**
 * A React component that also exposes a `getValue()` function to retrieve the user-supplied sink parameter (or null if the user faile to supply a value)
 */
type TickSinkParam<T> = React.Component & {getValue: () => ?T};

/**
 * A function that, given the name of the parameter, returns a `TickSinkParam` for it.
 */
type TickSinkParamGenerator<T> = (clearName: string, jsonName: string) => TickSinkParam<T>;

/**
 * All parameter generator functions available for tick sinks to use in their components
 */
const tickSinkParamGens: { [key: string]: TickSinkParamGenerator<any> } = {
  str: (clearName: string, jsonName: string): TickSinkParam<string> => {
    class StringInput extends React.Component {
      constructor(props) {
        super(props);
        this.state = {
          input: <Input />,
        };
      }

      getValue(): ?string {
        if(this.state.input.value == '') {
          return null;
        }

        return JSON.parse(`{${this.props.jsonName}: ${this.state.input.value}}`);
      }

      render() {
        return (
          <div key={clearName}>
            {this.props.clearName}{':'} {this.state.input}
          </div>
        );
      }
    }

    StringInput.propTypes = {
      clearName: React.PropTypes.string,
      jsonName: React.PropTypes.string,
    };

    return <StringInput name={name} />;
  }
};

/**
 * A set of definitions for props that represent all possible places that ticks can be sent to.  They contain the names of all paramaters that are required
 * in order to build the `HistTickDst`s internally.
 */
const tickSinkDefs: { [key: string]: Array<{clearName: string, paramType: TickSinkParam}> } = {
  Console: [],
  Flatfile: [{clearName: 'Filename', param: 'str'}],
  Postgres: [{clearName: 'Table', param: 'str'}],
  RedisChannel: [
    {clearName: 'Redis Host', param: 'str'},
    {clearName: 'Channel', param: 'str'},
  ],
  RedisSet: [
    {clearName: 'Redis Host', param: 'str'},
    {clearName: 'Set Name', param: 'str'},
  ]
};

export default {
  dataDownloaders: dataDownloaders,
  TickSink: TickSink,
  tickSinkDefs: tickSinkDefs,
};
