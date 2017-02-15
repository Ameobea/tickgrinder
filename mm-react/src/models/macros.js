//! Declares state related to asynchronous macro action execution

import { select} from 'redux-saga/effects';

import { execMacro } from '../utils/spawner_macro';
import { dummyDispatch } from '../utils/commands';

export default {
  namespace: 'macros',

  state: {
    asyncMacroActions: [], // a list of macro actions to be handled when a response to a previos macro action's command is received
    definedMacros: [], // list of all macros defined by the user
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
      let asyncCbs = select((gstate) => gstate.platform_communication.asyncMacroActions);
      for(var i=0; i<asyncCbs.length; i++) {
        if(asyncCbs[i].uuid == msg.uuid) {
          // execute the callback and see if there's another
          let newMacro = asyncCbs[i](msg);
          if(newMacro) {
            execMacro(dummyDispatch(put), newMacro);
          }
        }
      }
    },

    // TODO: Create function to automatically fetch the list of defined macros the first time they are requested
  },
};
