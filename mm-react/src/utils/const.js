//! Declares platform constants and metadata used to send commands.

/**
 * Contains a list of all data downloaders and information about them including their description, name, and
 * the command used to spawn them.
 */
const dataDownloaders = [
  {
    name: 'FXCM Native Data Downloader',
    description: 'Uses the native FXCM C++ API to request historical ticks programatically.  Requires ' +
      'valid FXCM demo account credentials.',
    command: 'SpawnFxcmNativeDataDownloader',
  }, {
    name: 'FXCM Flatfile Data Downloader',
    description: 'Retrieves the FXCM-hosted flatfile archives containing historical tick data and processes ' +
      'them into CSV files.',
    command: 'SpawnFxcmFlatfileDataDownloader',
  }, {
    name: 'IEX Data Downloader',
    description: 'Uses the live data streams from the Investor\'s Exchange (IEX) to get live price data for US stocks.',
    command: 'SpawnIexDataDownloader',
  }, {
    name: 'Poloniex Data Downloader',
    description: 'Hooks into the Poloniex API to retrieve live streaming orderbook and trade updates as well as ' +
      'historical trade data.  Only supports writing to Flatfile.',
    command: 'SpawnPoloniexDataDownloader',
  }
];

export default {
  dataDownloaders: dataDownloaders,
};
