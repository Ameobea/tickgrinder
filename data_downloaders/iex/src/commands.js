//! Functions useful for interfacing with the platform's command and response communication system
/* global WebSocket */
// @flow

const CONF = require('./conf.js');

// TODO: Swap to using the functions from the JS util library

/**
 * Generates a new V4 UUID in hyphenated form
 */
export function v4(): string {
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
 */
export function initWs(callback: (msg: {uuid: string, msg: any}) => void, ourUuid: string): WebSocket {
  let socketUrl = 'ws://localhost:7037';
  let socket = new WebSocket(socketUrl);
  socket.onmessage = (message: any) => {
    let parsed = JSON.parse(message.data);
    // throw away messages we're transmitting to channels we don't care about
    if ([CONF.redis_control_channel, CONF.redis_responses_channel, CONF.redis_log_channel, ourUuid].indexOf(parsed.channel) !== -1) {
      callback(parsed);
    }
  };

  socket.onerror = () => {
    // TODO
  };

  return socket;
}

/**
 * Processes a command from the platform and returns a response to be sent back, also taking any effects that the command commands us to take.
 */
export function handleCommand(command: any, our_uuid: string): any {
  switch (command) {
  case 'Ping':
    var temp = [our_uuid];
    return {Pong: {args: temp.splice(2)}};
  case 'Kill':
    return {Error: {status: 'We\'re client side, we don\'t take orders from you.'}};
  case 'Type':
    return {Info: {info: 'IEX Data Downloader'}};
  default:
    return {Error: {status: 'Command not recognized.'}};
  }
}
