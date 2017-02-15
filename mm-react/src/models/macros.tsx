//! Declares state related to asynchronous macro action execution

import { put, select} from 'redux-saga/effects';

import { execMacro } from '../utils/spawner_macro';

export default {
  namespace: "macros",

  state: {
    asyncMacroActions: [], // a list of macro actions to be handled when a response to a previos macro action's command is received
  },

  reducers: {
     /// register a new async callback to be executed when responses of a particular uuid are received
    registerMacroActionCb(state, {uuid, cb}) {
      return {...state,
        asyncMacroActions: [...state.asyncMacroActions, {uuid: uuid, cb: cb}],
      };
    },
  },

  effects: {
    /// Called as a callback to responses received from macro actions
    *asyncMacroAction({msg}, {call, put}) {
      // create a dummy `dispatch` function to pass to any returned actions
      function *dummyDispatch(args) {
        yield put(args);
      };

      let asyncCbs = select((gstate) => gstate.platform_communication.asyncMacroActions);
      for(var i=0; i<asyncCbs.length; i++) {
        if(asyncCbs[i].uuid == msg.uuid) {
          // execute the callback and see if there's another
          let newMacro = asyncCbs[i](msg);
          if(newMacro) {
            execMacro(dummyDispatch, newMacro);
          }
        }
      }
    },
  },
}
