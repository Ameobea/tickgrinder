//! Contains state related to the living instances in the platform and actions responsible for handling
//! when new instances spawn/are killed.

export default {
  namespace: 'instances',

  state: {
    living_instances: [],
  },

  reducers: {
    /// called when a new instance is spawned and should be added to the list of living instances
    instanceSpawned(state, action) {
      return {...state,
        living_instances: [...state.living_instances, action.instance],
      };
    },

    /// called when an instance has been killed or communication with that instance has been lost
    instanceKilled(state, action) {
      return {...state,
        living_instances: state.living_instances.filter(instance => instance.uuid != action.instance.uuid),
      };
    },
  },
}
