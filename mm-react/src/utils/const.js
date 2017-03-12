//! Declares platform constants and metadata used to send commands.
// @flow

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
 * A set of props that represent all possible places that ticks can be sent to.  They contain the paramaters that are required
 * in order to build the `HistTickDst`s internally and are designed to be placed in forms.
 */
const tickSinks = {
  // TODO
};

export default {
  dataDownloaders: dataDownloaders,
  tickSinks: tickSinks,
};
