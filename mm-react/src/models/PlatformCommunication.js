//! Functions for communication with the platform's modules using Websockets that are bridged to Redis pub/sub.

const CONF = require('../conf');
import { initWs, getResponse } from '../utils/commands';

const handleMessage = (dispatch, {uuid, channel, message}) => {
  let message_str = message.replace(/{("\w*")}/g, "$1");
  let msg = JSON.parse(message);

  // dispatch the message to the corresponding reducer depending on its type
  if(msg.cmd) {
    if(msg.cmd.Log) {
      dispatch({type: 'logReceived', msg: msg});
    } else {
      dispatch({type: 'commandReceived', msg: msg});
    }
  } else {
    dispatch({type: 'responseReceived', msg: msg});
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
      if(state.commands.length > CONF.mm_cache_size) {
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
      if(state.responses.length > CONF.mm_cache_size) {
        state.responses.pop();
      }

      return state;
    },

    // adds a log message to the state, removing the oldest ond if the buffer is larger than the size limit
    logReceived(state = {log_messages: []}, action) {
      let new_state =  { ...state,
        log_messages: [...state.log_messages, action.msg],
      };

      // trim the oldest line out of the cache to keep it under the limit if it's over the limit
      if(new_state.log_messages.length > CONF.mm_cache_size) {
        new_state.log_messages.pop();
      }

      return new_state;
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
