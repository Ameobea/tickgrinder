//! Functions for communication with the platform's modules using Websockets that are bridged to Redis pub/sub.

const CONF = require('../conf');
import { initWs, getResponse } from '../utils/commands';

const handleMessage = (dispatch, {uuid, channel, message}) => {
  let message_str = message.replace(/{("\w*")}/g, "$1");
  let msg = JSON.parse(message);

  // dispatch the message to the corresponding reducer depending on its type
  if(msg.cmd) {
    if(msg.cmd.Log) {
      dispatch({type: 'logReceived', msg: msg.cmd.Log.msg});
    } else {
      dispatch({type: 'commandReceived', msg: msg.cmd});
    }
  } else {
    dispatch({type: 'responseReceived', msg: msg.res});
  }
};

export default {
  namespace: 'platform_communication',

  state: {
    log_messages: [],
    commands: [],
    responses: [],
  },

  reducers: {
    // receives the websocket object from the `redisListener` subscription after it has initialized the websocket connection
    websocketConnected(state = {}, action) {
      state.socket = action.socket;
      return state;
    },

    // adds the command to the state, removing the oldest one if the buffer is larger than the size limit
    commandReceived(state = {commands: []}, action) {
      state.commands.push(action.msg);
      if(state.commands.length > 25000) { // TODO: Create config setting for this
        state.commands.pop();
      }

      // Get a response to send back in reply to the received command
      let res = getResponse(action.msg);
      // TODO: Send the message

      return state;
    },

    // adds a response to the state, removing the oldest one if the buffer is larger than the size limit
    responseReceived(state = {}, action) {
      state.responses.push(action.msg);
      if(state.responses.length > 25000) { // TODO: Create config setting for this
        state.responses.pop();
      }

      return state;
    },

    // adds a log message to the state, removing the oldest ond if the buffer is larger than the size limit
    logReceived(state = {log_messages: []}, action) {
      state.log_messages.push(action.msg);
      if(state.log_messages.length > 25000) { // TODO: Create config setting for this
        state.log_messages.pop();
      }

      return state;
    },
  },

  subscriptions: {
    redisListener({dispatch, history}) {
      // initialize redis clients for sending and receiving messages
      let socket = initWs(handleMessage, dispatch);
      // save the socket to the inner state so we can use it to send as well
      dispatch({type: 'websocketConnected', action: {socket: socket}});
    },
  },
}
