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
 */
class TickSink extends React.Component {
  constructor(props) {
    super(props);
    // create `TickSink`s out of all of the supplied sink definitions
    let defs = props.paramDefs;
    const sinks = defs.map((def: {paramName: string, paramType: string}): TickSinkParam<any> => tickSinkParams[def.paramType](def.paramName));

    this.state = {
      sinks: sinks,
    };
  }

  render() {

  }
}

TickSink.PropTypes = {
  name: React.PropTypes.string,
  paramDefs: React.PropTypes.arrayOf(
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
type TickSinkParamGenerator<T> = (name: string) => TickSinkParam<T>;

/**
 * All parameter generator functions available for tick sinks to use in their components
 */
const tickSinkParamGens: { [key: string]: TickSinkParamGenerator<any> } = {
  str: (name: string): TickSinkParam<string> => {
    class StringInput extends React.Component {
      getValue(): ?string {
        return 'TEST';
      }

      render() {
        return (
          <div>
            {this.props.name}{':'} <Input />
          </div>
        );
      }
    }

    StringInput.propTypes = {
      name: React.PropTypes.string,
    };

    return <StringInput name={name} />;
  }
};

/**
 * A set of definitions for props that represent all possible places that ticks can be sent to.  They contain the names of all paramaters that are required
 * in order to build the `HistTickDst`s internally.
 */
const tickSinkDefs: { [key: string]: Array<{name: string, param: TickSinkParam}> } = {
  Console: [],
  Flatfile: [{name: 'Filename', param: tickSinkParamGens.str}],
};

export default {
  dataDownloaders: dataDownloaders,
  tickSinks: tickSinks,
};
