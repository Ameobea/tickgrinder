//! Functions for communication with the platform's modules using Websockets that are bridged to Redis pub/sub.

const CONF = require('../conf');
import { initWs, getResponse, v4 } from '../utils/commands';

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
    socket: undefined,
    uuid: v4(),
  },

  reducers: {
    // receives the websocket object from the `redisListener` subscription after it has initialized the websocket connection
    websocketConnected(state, action) {
      return {...state,
        socket: action.socket,
      };
    },

    // called to set a UUID for the platform during initialization
    setUuid(state, action) {
      return {...state,
        uuid: action.uuid,
      };
    },

    // adds the command to the state, removing the oldest one if the buffer is larger than the size limit
    commandReceived(state, action) {
      state.commands.push(action.msg);
      if(state.commands.length > CONF.mm_cache_size) {
        state.commands.pop();
      }

      // Get a response to send back in reply to the received command
      let res = getResponse(action.msg, state.uuid);
      // format the response message and send it over the websocket connection to be proxied to Redis
      let uuid = v4();
      let wrapped_res = {uuid: uuid, res: res};
      let wsmsg = {uuid: uuid, channel: CONF.redis_responses_channel, message: JSON.stringify(wrapped_res)};
      state.socket.send(JSON.stringify(wsmsg));

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
      // generate and set a UUID for the MM
      let our_uuid = v4();
      dispatch({type: 'setUuid', uuid: our_uuid});
      // send a `Ready` message to Redis to let the platform know we're here
      socket.onopen = (evt) => {
        let uuid = v4();
        let ready_msg = {uuid: uuid, cmd: {Ready: {instance_type: "MM", uuid: our_uuid}}};
        let wsmsg = {uuid: uuid, channel: CONF.redis_control_channel, message: JSON.stringify(ready_msg)}
        socket.send(JSON.stringify(wsmsg));
      }
      // save the socket to the inner state so we can use it to send as well
      dispatch({type: 'websocketConnected', socket: socket});
    },
  },
}
