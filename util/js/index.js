//! JavaScript utility library.  See README.txt for more information.
// @flow

const ffi = require('./src/ffi');
const CONF = require('./src/conf');

/**
 * Generates a new V4 UUID in hyphenated form
 */
function v4(): string {
  function s4(): string {
    return Math.floor((1 + Math.random()) * 0x10000)
      .toString(16)
      .substring(1);
  }
  return s4() + s4() + '-' + s4() + '-' + s4() + '-' +
    s4() + '-' + s4() + s4() + s4();
}

/**
 * Starts the WS listening for new messages sets up processing callback
 * @param {function} callback - The function invoked when a message is received over the websocket connection
 * @param {dispatch} dispatch - A piece of state passed along with the message to the callback function.
 * @param {string} ourUuid - The UUID of the instance creating this websocket server
 * @param {function} wsError - A callback invoked when the websocket encounters an error
 */
function initWs(callback: (dispatch: any, parsed: any) => void, dispatch: any, ourUuid: string, wsError: (e: string) => void) {
  let socketUrl = 'ws://localhost:7037';
  let socket = new WebSocket(socketUrl);
  socket.onmessage = message => {
    if(typeof(message.data) != "string") {
      wsError('Received non-string data over websocket connection!');
      return;
    }
    let parsed = JSON.parse(message.data);
    // throw away messages we're transmitting to channels we don't care about
    if ([CONF.redis_control_channel, CONF.redis_responses_channel, CONF.redis_log_channel, ourUuid].indexOf(parsed.channel) !== -1) {
      callback(dispatch, parsed);
    }
  };

  socket.onerror = () => {
    wsError('Unhandled error occured on the websocket connection!');
  };

  return socket;
}

module.exports = {
  ffi: ffi,
  v4: v4,
  initWs: initWs,
};
