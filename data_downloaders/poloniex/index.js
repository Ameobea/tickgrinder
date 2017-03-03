//! Data downloader that connects to the Poloniex Websocket API to record live streaming DOM data

const autobahn = require('autobahn');
const wsuri = 'wss://api.poloniex.com';

// creates a new connection to the API endpoint
var connection = new autobahn.Connection({
  url: wsuri,
  realm: 'realm1'
});

connection.onopen = session => {
  function marketEvent (args,kwargs) {
    console.log(args);
	}

	function tickerEvent (args,kwargs) {
		console.log(args);
	}

	function trollboxEvent (args,kwargs) {
		console.log(args);
	}

	session.subscribe('BTC_XMR', marketEvent);
	session.subscribe('ticker', tickerEvent);
	session.subscribe('trollbox', trollboxEvent);
}

connection.onclose = function () {
  console.log('Websocket connection closed');
}

connection.open();
