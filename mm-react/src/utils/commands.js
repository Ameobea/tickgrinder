//! Functions for interfacing with the platform's Redis-based communication system
/* eslint-env browser */

import React from 'react';
import { Modal } from 'antd';

const CONF = require('../conf.js');

/**
 * Creates a big error modal if the MM can't connect to the websocket proxy.
 */
const wsError = () => {
  Modal.error({
    closable: false,
    title: 'Unable to connect to Websocket server!',
    content: 'In order to use this interface, the Redis<->Websocket proxy must be running.  \n\n' +
      'To start it, run.sh in the project root directory and refresh this page.'
  });
};

/**
 * Generates a new V4 UUID in hyphenated form
 */
function v4 () {
  function s4 () {
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
function initWs (callback, dispatch, ourUuid) {
  let socketUrl = 'ws://localhost:7037';
  let socket = new WebSocket(socketUrl);
  socket.onmessage = message => {
    let parsed = JSON.parse(message.data);
    // throw away messages we're transmitting to channels we don't care about
    if ([CONF.redis_control_channel, CONF.redis_responses_channel, CONF.redis_log_channel, ourUuid].indexOf(parsed.channel) !== -1) {
      callback(dispatch, parsed);
    }
  };

  socket.onerror = () => {
    wsError();
  };

  return socket;
}

/**
 * Processes a command and returns a Response to send back
 */
function getResponse (command, uuid) {
  let res, action;
  switch (command) {
  case 'Ping':
    var temp = [uuid];
    res = {Pong: {args: temp.splice(2)}};
    break;
  case 'Kill':
    res = {Error: {status: 'We\'re client side, we don\'t take orders from you.'}};
    break;
  case 'Type':
    res = {Info: {info: 'MM'}};
    break;
  default:
    if (command.Ready) {
      res = 'Ok';
      action = 'instances/instanceSpawned';
    } else {
      res = {Error: {status: 'Command not recognized.'}};
    }
    break;
  }

  return {res: res, action: action};
}

/**
 * Given the list of all currently running instances, returns the UUIDs of all instances with the specified name.
 *  Returns an empty list if there are no living instances with the specified name.
 */
const getInstance = (name, living_instances) => {
  return living_instances.filter(inst => inst.instance_type === name);
};

/**
 * create a dummy `dispatch` function to pass to any returned actions
 */
function *dummyDispatch (put) {
  return function* (args) { yield put(args); };
}

/**
 * defines the shape of a `WrappedCommand` for use in `PropTypes` declarations
 */
const WrappedCommand = {
  uuid: React.PropTypes.string.isRequired,
  cmd: React.PropTypes.any.isRequired,
};

/**
 * defines the shape of a `WrappedResponse` for use in `PropTypes` declarations
 */
const WrappedResponse ={
  uuid: React.PropTypes.string.isRequired,
  cmd: React.PropTypes.any.isRequired,
};

/**
 * defines the shape of an `Instance` for use in `PropTypes` declarations
 */
const InstanceShape = {
  uuid: React.PropTypes.string.isRequired,
  instance_type: React.PropTypes.string.isRequired,
};

/**
 * defines the shape of a `HistTickDst` for use in `PropTypes` declarations.  Corresponds to the Rust
 * `HistTickDst` enum.
 */
const HistTickDstShape = {
  Flatfile: React.PropTypes.shape({
    filename: React.PropTypes.string.isRequired,
  }),
  Postgres: React.PropTypes.shape({
    table: React.PropTypes.string.isRequired,
  }),
  RedisChannel: React.PropTypes.shape({
    host: React.PropTypes.string.isRequired,
    channel: React.PropTypes.string.isRequired,
  }),
  RedisSet: React.PropTypes.shape({
    host: React.PropTypes.string.isRequired,
    set_name: React.PropTypes.string.isRequired,
  }),
  Console: React.PropTypes.string,
};

/**
 * defines the shape of a `Document` for use in `PropTypes` declarations
 */
const DocumentShape = {
  title: React.PropTypes.string.isRequired,
  body: React.PropTypes.string.isRequired,
  tags: React.PropTypes.arrayOf(React.PropTypes.string),
  creation_date: React.PropTypes.string.isRequired,
  modification_date: React.PropTypes.string.isRequired,
  id: React.PropTypes.string.isRequired,
};

export default {
  initWs: initWs,
  getResponse: getResponse,
  v4: v4,
  getInstance: getInstance,
  dummyDispatch: dummyDispatch,
  WrappedCommand: WrappedCommand,
  WrappedResponse: WrappedResponse,
  InstanceShape: InstanceShape,
  DocumentShape: DocumentShape,
  HistTickDstShape: HistTickDstShape,
};
