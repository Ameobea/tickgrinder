//! Contains state related to the living instances in the platform and actions responsible for handling
//! when new instances spawn/are killed.

import { message } from 'antd';

export default {
  namespace: 'instances',

  state: {
    living_instances: [],
    selected_spawn_opt: {name: "Backtester", cmd: "SpawnBacktester"},
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
        message.error("Error killing instance: \"" + msg.res.Error.status + "\"", 4);
      }

      // no modifications to state at this point, so just return it
      return {...state};
    },

    /// Called when the selected instance in the spawn instance dropdown menu is changed
    instanceSpawnChanged(state, {name, cmd}) {
      return {...state,
        selected_spawn_opt: {name: name, cmd: cmd},
      };
    },

    /// Called when a successful response is received from the spawner as a result from a request to manually spawn an instance
    instanceSpawnCallback(state, {msg}) {
      console.log(msg);
      // display a popup notificaiton of the result of the spawn action
      if(msg.res == "Ok") {
        message.info("Instance spawn request accepted; instance now spawning.", 3);
      } else if(msg.res.Error) {
        message.error("Instance spawn request rejected: \"" + msg.res.Error.status + "\"", 3);
      } else {
        message.error("Received unexpected response from Spawner: " + JSON.stringify(msg.res));
      }

      // no modifications to state needed; the spawned instance will transmit a `Ready` message which will be handled elsewhere
      return {...state};
    }
  },
}
