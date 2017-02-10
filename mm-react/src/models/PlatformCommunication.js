//! Functions for communication with the platform's modules using Websockets that are bridged to Redis pub/sub.

import { delay } from 'redux-saga';
import { put, select } from 'redux-saga/effects';

const CONF = require('../conf');
import { initWs, getResponse, v4 } from '../utils/commands';

const handleMessage = (dispatch, {uuid, channel, message}) => {
  let message_str = message.replace(/{("\w*")}/g, "$1");
  let msg = JSON.parse(message);

  // dispatch the message to the corresponding reducer depending on its type
  if(msg.cmd) {
    if(msg.cmd.Log) { // is log message
      dispatch({type: 'logReceived', msg: msg});
    } else { // is a command
      dispatch({type: 'commandReceived', msg: msg});
    }
  } else { // is a response
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
    interest_list: [], // list of UUIDs of responses we're interested in and callbacks to run for when they're received
  },

  reducers: {
    /// sends a command and executes a callback for all responses received.  The callback should accept two parameters:
    /// `put` and `msg` in that order like `(put, res) => {...}`.
    transmitCommand(state, {channel, cmd, cb, uuid}) {
      // if no callback was supplied, set it to a no-op cb
      if(cb == undefined) {
        cb = () => {};
      }

      // broadcast the command over the socket
      let msg = {uuid: uuid, cmd: cmd};
      let wsmsg = {uuid: uuid, channel: channel, message: JSON.stringify(msg)};
      state.socket.send(JSON.stringify(wsmsg));

      // add the uuid and callback to the list of monitored UUIDs
      return {...state,
        interest_list: [...state.interest_list, {uuid: uuid, cb: cb}],
      };
    },

    /// Called after a timeout period; deregisters interest in a UUID.
    deregisterInterest(state, {uuid}) {
      return {...state,
        interest_list: state.interest_list.filter(interest => interest.uuid != uuid),
      };
    },

    /// receives the websocket object from the `redisListener` subscription after it has initialized the websocket connection
    websocketConnected(state, action) {
      return {...state,
        socket: action.socket,
      };
    },

    /// called to set a UUID for the platform during initialization
    setUuid(state, action) {
      return {...state,
        uuid: action.uuid,
      };
    },

    /// adds the command to the state, removing the oldest one if the buffer is larger than the size limit
    commandReceived(state, action) {
      let new_state = {...state,
        commands: [...state.commands, action.msg],
      };
      if(new_state.commands.length > CONF.mm_cache_size) {
        new_state.commands.pop();
      }

      // Get a response to send back in reply to the received command
      let res = getResponse(action.msg.cmd, new_state.uuid);
      // format the response message and send it over the websocket connection to be proxied to Redis
      let uuid = v4();
      let wrapped_res = {uuid: action.msg.uuid, res: res};
      let wsmsg = {uuid: action.msg.uuid, channel: CONF.redis_responses_channel, message: JSON.stringify(wrapped_res)};
      new_state.socket.send(JSON.stringify(wsmsg));

      return new_state;
    },

    /// adds a response to the state, removing the oldest one if the buffer is larger than the size limit
    addResponse(state = {}, {msg}) {
      let new_state = {...state,
        responses: [...state.responses, msg],
      };
      if(new_state.responses.length > CONF.mm_cache_size) {
        new_state.responses.pop();
      }

      return new_state;
    },

    /// adds a log message to the state, removing the oldest ond if the buffer is larger than the size limit
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

  effects: {
    /// Initializes the timeout for deregistering that interest and dispatches the action to transmit
    /// the command over the WebSocket
    *sendCommand(action, {channel, cmd, cb}) {
      // transmit the command and register interest in responses with its UUID
      let uuid = v4();
      yield put({type: 'platform_communication/transmitCommand', channel: channel, cmd: cmd, cb: cb, uuid: uuid});
      // give instances a chance to receive + process the command and transmit their responses
      yield delay(3000);
      // then deregister interest in the command's uuid
      yield put({type: 'platform_communication/deregisterInterest', uuid: uuid})
    },

    /// Called when responses are received.  Invokes the interest checker.
    *responseReceived(action, {call, put}) {
      yield put({type: 'addResponse', msg: action.msg});

      // check if there is interest registered in the responses's UUID and calls its callback if there is.
      // get the interest list from the state
      let interest_list = yield select(gstate => gstate.platform_communication.interest_list);
      let matched = interest_list.filter(interest => interest.uuid == action.msg.uuid);
      // invoke the callbacks of all registered interests
      for(var i=0; i<matched.length; i++) {
        matched[i].cb(put, matched[i].res);
      }
    },
  },

  subscriptions: {
    redisListener({dispatch, history}) {
      // initialize redis clients for sending and receiving messages
      let socket = initWs(handleMessage, dispatch);
      // generate and set a UUID for the MM
      let our_uuid = v4();
      dispatch({type: 'setUuid', uuid: our_uuid});

      socket.onopen = (evt) => {
        // send a `Ready` message to Redis to let the platform know we're here
        let cmd = {Ready: {instance_type: "MM", uuid: our_uuid}};
        dispatch({type: 'sendCommand', channel: CONF.redis_control_channel, cmd: cmd})

        let censusCb = (dispatch, res) => {

          console.log(res);
          put({type: 'censusReceived', census: res.Info.info});
        };

        setTimeout(() => {
          // send a `Census` message to get an initial picture of the platform's population;
          dispatch({type: 'sendCommand', channel: CONF.redis_control_channel, cmd: "Census", cb: censusCb});
        }, 300);
      }
      // save the socket to the inner state so we can use it to send as well
      dispatch({type: 'websocketConnected', socket: socket});
    },
  },
}
