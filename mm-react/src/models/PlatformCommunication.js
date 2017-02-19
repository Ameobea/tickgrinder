//! Functions for communication with the platform's modules using Websockets that are bridged to Redis pub/sub.

import { delay } from 'redux-saga';
import { select } from 'redux-saga/effects';
import { message } from 'antd';

const CONF = require('../conf');
import { initWs, getResponse, getInstance, v4, dummyDispatch } from '../utils/commands';

const handleMessage = (dispatch, {uuid, channel, message}) => {
  let msg = JSON.parse(message);

  // dispatch the message to the corresponding reducer depending on its type
  if (msg.cmd) {
    if (msg.cmd.Log) { // is log message
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
    interestList: [], // list of UUIDs of responses we're interested in and callbacks to run for when they're received
    queryResults: [], // list of all document titles returned in response to a query
    docQueryCbs: [], // list of functions that are called with the list of matched titles every time a query response is received
  },

  reducers: {
    /**
     * sends a command and executes a callback for all responses received.  The callback should accept two parameters:
     * `put` and `msg` in that order like `(put, msg) => {...}`.
     */
    transmitCommand (state, {channel, cmd, cb_action, uuid}) {
      // broadcast the command over the socket
      let msg = {uuid: uuid, cmd: cmd};
      let wsmsg = {uuid: uuid, channel: channel, message: JSON.stringify(msg)};
      state.socket.send(JSON.stringify(wsmsg));

      // add the uuid and callback to the list of monitored UUIDs
      return {...state,
        interestList: [...state.interestList, {uuid: uuid, cb_action: cb_action}]
      };
    },

    /**
     * sends a response over the specified redic channel+uuid over the WebSocket to the platform.
     */
    transmitResponse (state, {uuid, res}) {
      let msg = {uuid: uuid, res: res};
      let wsmsg = {uuid: uuid, channel: CONF.redis_responses_channel, message: JSON.stringify(msg)};
      state.socket.send(JSON.stringify(wsmsg));

      // we don't actually modify the state at all, ju/st use the socket
      return {...state};
    },

    /**
     * Called after a timeout period; deregisters interest in a UUID.
     */
    deregisterInterest (state, {uuid}) {
      return {...state,
        interestList: state.interestList.filter(interest => interest.uuid !== uuid)
      };
    },

    /**
     * receives the websocket object from the `redisListener` subscription after it has initialized the websocket connection
     */
    websocketConnected (state, {socket}) {
      return {...state,
        socket: socket
      };
    },

    /**
     * called to set a UUID for the platform during initialization
     */
    setUuid (state, {uuid}) {
      return {...state,
        uuid: uuid
      };
    },

    /**
     * adds a response to the state, removing the oldest one if the buffer is larger than the size limit
     */
    addResponse (state = {}, {msg}) {
      let newState = {...state,
        responses: [...state.responses, msg]
      };

      if (newState.responses.length > CONF.mm_cache_size) {
        newState.responses.pop();
      }

      return newState;
    },

    /**
     * adds a log message to the state, removing the oldest ond if the buffer is larger than the size limit
     */
    logReceived (state, {msg}) {
      let newState = {...state,
        log_messages: [...state.log_messages, msg]
      };

      // trim the oldest line out of the cache to keep it under the limit if it's over the limit
      if (newState.log_messages.length > CONF.mm_cache_size) {
        newState.log_messages.pop();
      }

      return newState;
    },

    /**
     * Called when a postgres query is sent; registers the a callback to be executed when the response is received.
     */
    postgresQuerySent (state, {query, cb}) {
      return {...state,
        postgresCbs: [...state.postgresCbs, {query: query, cb: cb}]
      };
    },

    /**
     * Registers a callback to be executed every time a new `DocQueryResponse` is received.
     */
    registerDocQueryReceiver (state, {cb}) {
      return {...state,
        docQueryCbs: [...state.docQueryCbs, cb]
      };
    },

    docQueryResponseReceived (state, {msg}) {
      let matchedDocs;
      if(msg.res.DocumentQueryResult) {
        matchedDocs = msg.res.DocumentQueryResult.results.map(o => JSON.parse(o).title[0]);

        // execute all registered callbacks
        for(var i=0; i<state.docQueryCbs.length; i++) {
          state.docQueryCbs[i](matchedDocs);
        }

        return {...state,
          queryResults: matchedDocs,
        };
      } else if(msg.res.Error) {
        message.error('Error while processing query: ' + msg.res.Error.status);
      } else {
        message.error('Unknown error occured while processing query: ' + JSON.stringify(msg));
      }

      return {...state};
    }
  },

  effects: {
    /**
     * Initializes the timeout for deregistering that interest and dispatches the action to transmit
     * the command over the WebSocket.  `cb_action` is the name of the action/effect that will be triggered for all
     * responses received with UUIDs matching that of the sent command; they're essentially callbacks.
     */
    *sendCommandWithUuid ({channel, cmd, cb_action, uuid}, {call, put}) {
      // transmit the command and register interest in responses with its UUID
      yield put({type: 'transmitCommand', channel: channel, cmd: cmd, cb_action: cb_action, uuid: uuid});
      // give instances a chance to receive + process the command and transmit their responses
      yield delay(3000);
      // then deregister interest in the command's uuid
      yield put({type: 'deregisterInterest', uuid: uuid});
    },

    /**
     * A wrapper for `sendCommandWithUuid` that automatically generates a UUID
     */
    *sendCommand ({channel, cmd, cb_action}, {call, put}) {
      // generate a random uuid for use in the command
      let uuid = v4();
      yield put({type: 'sendCommandWithUuid', channel: channel, cmd: cmd, cb_action: cb_action, uuid: uuid});
    },

    /**
     * Called when responses are received.  Invokes the interest checker.
     */
    *responseReceived ({msg}, {call, put}) {
      // add the response to the list of cached responses
      yield put({type: 'addResponse', msg: msg});

      // check if there is interest registered in the responses's UUID and calls its callback if there is.
      // get the interest list from the state
      let interestList = yield select(gstate => gstate.platform_communication.interestList);
      let matched = interestList.filter(interest => interest.uuid === msg.uuid);
      // invoke the callbacks of all registered interests
      for (var i = 0; i < matched.length; i++) {
        if (matched[i].cb_action) {
          yield put({type: matched[i].cb_action, msg: msg});
        }
      }
    },

    /**
     * Sends a document query to the Tantivy-backed document store.  Registers interest in responses with the UUID
     * of the sent query and handles the responses by updating the `queryResults` state.
     */
    *sendDocQuery ({query}, {call, put}) {
      let cmd = {QueryDocumentStore: {query: query}};
      yield put({
        type: 'sendCommandToInstance',
        cb_action: 'docQueryResponseReceived',
        cmd: cmd,
        instance_name: 'Spawner',
      });
    },

    /*
     * adds the command to the state, removing the oldest one if the buffer is larger than the size limit
     */
    *commandReceived ({msg}, {call, put}) {
      // Get a response to send back in reply to the received command
      let {res, action} = getResponse(msg.cmd, msg.uuid);
      yield put({type: 'transmitResponse', uuid: msg.uuid, res: res});

      // if an action was supplied, execute it
      if (action) {
        yield put({type: action, msg: msg});
      }
    },

    /**
     * sends a command to the first instance (selected arbitrarily) with the specified `instanceType`.
     */
    *sendCommandToInstance ({cmd, cb_action, instance_name}, {call, put}) {
      let living_instances = yield select(gstate => gstate.instances.living_instances);
      let instanceUuid = getInstance(instance_name, living_instances)[0].uuid;

      // actually send the command to the instance
      yield put({type: 'sendCommand', channel: instanceUuid, cb_action: cb_action, cmd: cmd});
    },

    /**
     * sends a command to the first instance (selected arbitrarily) with the specified `instanceType`.
     *  Same as `sendCommandToInstance` but lets you supply a UUID for the command to be sent.
     */
    *sendCommandToInstanceWithUuid ({cmd, cb_action, instance_name, uuid}, {call, put}) {
      let living_instances = yield select(gstate => gstate.instances.living_instances);
      let instanceUuid = getInstance(instance_name, living_instances).uuid;

      // actually send the command to the instance
      yield put({type: 'sendCommandWithUuid', channel: instanceUuid, cb_action: cb_action, cmd: cmd, uuid: uuid});
    },

    /**
     * sends a PostgreSQL query to the spawner instance to be executed.
     */
    *postgresQuery ({query, cb}, {call, put}) {
      let cmd = {PostgresQuery: {
        query: query
      }};

      let uuid = v4();
      yield put({type: 'postgresQuerySend', uuid: uuid, cb: cb});
      yield put({
        type: 'sendCommandToInstanceWithUuid',
        cb_action: 'postgresResponse',
        uuid: uuid,
        cmd: cmd,
        instance_name: 'Spawner'
      });
    },

    /**
     * Called when a response to a postgres query is received.  Executes the callback with a simulated dispatch function and
     * removes the callback from the list of pending callbacks.
     */
    *postgresResponseReceived ({msg}, {call, put}) {
      let pendingCbs = select(gstate => gstate.platform_communication.postgresCbs);
      for (var i = 0; i < pendingCbs.length; i++) {
        if (pendingCbs[i].uuid === msg.uuid) {
          // if the uuids match, call the pending callback with the dummy dispatch function
          yield pendingCbs[i].cb(dummyDispatch(put), msg);
        }
      }
    },

    /**
     * sends a log message over the log channel.  Severity is a number from 0-4 corresponding to the levels DEBUG to CRITICAL.
     */
    *log ({label, msg, severity}, {call, put}) {
      // create function to convert numeric severity into string
      let convertSeverity = numSev => {
        switch (numSev) {
        case (0):
          return 'Debug';
        case (1):
          return 'Notice';
        case (2):
          return 'Warning';
        case (3):
          return 'Error';
        case (4):
          return 'Critical';
        }
      };

      // assign a default label if one was not supplied
      if (!label) {
        label = 'General';
      }

      // create the `Log` command
      let ourUuid = yield select(gstate => gstate.platform_communication.uuid);
      let logCmd = {Log: {
        sender: {
          instance_uuid: ourUuid, instance_type: 'MM'
        },
        message_type: label,
        message: msg,
        level: convertSeverity(severity)
      }};

      // dispatch a command to log the message
      yield put({type: 'sendCommand', channel: CONF.redis_log_channel, cmd: logCmd});
    }
  },

  subscriptions: {
    redisListener ({dispatch, history}) {
      let ourUuid = v4();
      // initialize redis clients for sending and receiving messages
      let socket = initWs(handleMessage, dispatch, ourUuid);
      // generate and set a UUID for the MM
      dispatch({type: 'setUuid', uuid: ourUuid});

      socket.onopen = (evt) => {
        // send a `Ready` message to Redis to let the platform know we're here
        let cmd = {Ready: {instance_type: 'MM', uuid: ourUuid}};
        dispatch({type: 'sendCommand', channel: CONF.redis_control_channel, cmd: cmd});

        setTimeout(() => {
          // send a `Census` message to get an initial picture of the platform's population;
          dispatch({type: 'sendCommand', channel: CONF.redis_control_channel, cmd: 'Census', cb_action: 'instances/censusReceived'});
        }, 300);
      };

      // save the socket to the inner state so we can use it to send as well
      dispatch({type: 'websocketConnected', socket: socket});
    }
  }
};
