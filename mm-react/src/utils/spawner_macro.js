//! Functions for dealing  with spawner macros

import { v4 } from '../utils/commands';

// Macros are in this format:
// { ...metadata,
//   actions: [{},{},],
// }
//
// `actions` is a list of macro actions that are evaluated in order.
// Macro actions have this format:
// { name: "Name of macro", // optional
//   description: "Description of macro", // optional
//   actionType: String identifying the type of action this is.
//               For now, the only kind accepted is "Command",
//   payload: {/*contains the command, query, etc. to be executed*/},
//   callbacks: [{macroCb}, {macroCb}, ...], optional
// }

// macro callbacks are functions that take in a response message and either return a macro action to evaluate or null.
// their Rust function signature would look like this:
// `fn macroCb(msg: WrappedResponse) -> Option<MacroAction> `

/// Executes a single spawner macro action
const execMacroAction = (dispatch, action) => {
  switch(action.actionType) {
    case("command"): // send a command to the platform
      let cmd_uuid = v4();
      // if there are callbacks, register them to be handled when responses from this cmd are received
      for(var i=0; i<action.callbacks.length; i++) {
        dispatch({
          type: 'platform_communication/registerMacroActionCb',
          uuid: cmd_uuid,
          cb: action.callbacks[i],
        });
      }

      // send the command to the platform
      dispatch({
        type: 'platform_communication/sendCommand',
        cmd: action.payload.cmd,
        channel: action.payload.channel,
        cb_action: 'asyncMacroAction',
      });
  }
};

/// Executes a spawner macro
const execMacro = (dispatch, macro) => {
  // send log message indicating that we're executing a macro
  dispatch({
    type: "platform_communication/log",
    label: "macroExecution",
    msg: "Executing macro " + macro.name,
    severity: 1,
  });

  // execute the macro's actions, one at a time in order
  for(var i=0; i<macro.actions.length; i++) {
    execMacroAction(macro.actions[i]);
  }
};

export default {
  execMacro: execMacro,
};
