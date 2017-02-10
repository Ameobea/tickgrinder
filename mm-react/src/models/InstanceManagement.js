//! Contains state related to the living instances in the platform and actions responsible for handling
//! when new instances spawn/are killed.

import { message } from 'antd';

export default {
  namespace: 'instances',

  state: {
    living_instances: [],
  },

  reducers: {
    /// called when a new instance is spawned and should be added to the list of living instances
    instanceSpawned(state, {msg}) {
      // don't add instances if we already know about them for some reason
      if(state.living_instances.filter(inst => inst.uuid == msg.cmd.Ready.uuid).length !== 0) {
        return {...state};
      }

      return {...state,
        living_instances: [...state.living_instances, msg.cmd.Ready],
      };
    },

    /// called when an instance has been killed or communication with that instance has been lost
    instanceKilled(state, action) {
      return {...state,
        living_instances: state.living_instances.filter(instance => instance.uuid != action.instance.uuid),
      };
    },

    /// Called when the platform receives census data from the spawner.
    censusReceived(state, {msg}) {
      if(!msg.res.Info) {
        return {...state};
      } else {
        return {...state,
          living_instances: JSON.parse(msg.res.Info.info),
        };
      }
    },

    /// Called when a response from the Instance a kill message was sent to is received
    instanceKillMessageReceived(state, {msg}) {
      // display a popup notification of the result of the kill command
      if(msg.res.Info) {
        message.info("Message received from instance: \"" + msg.res.Info.info + "\"", 4);
      } else {
        message.info("Error killing instance: \"" + msg.res.Error.status + "\"", 4);
      }

      // no modifications to state at this point, so just return it
      return {...state};
    },
  },
}
