//! Event handlers for actions on the logging page such as filtering messages.

export default {
  namespace: 'logging',

  state: {
    selected_categories: [],
    selected_severities: [],
    selected_instances: [],
    inclusive: false,
  },

  reducers: {
    /// handles clicks on the category tags in the `LiveLog`
    categoryAdded(state, action) {
      // only add if it's not already there
      if(state.selected_categories.indexOf(action.item) != -1) {
        return {...state};
      }

      return {...state,
        selected_categories: [...state.selected_categories, action.item],
      };
    },

    /// handles clicks on the close button for selected categories in `LiveLog`
    categoryClosed(state, action) {
      return {...state,
        selected_categories: state.selected_categories.filter(category => category != action.item),
      };
    },

    /// handles clicks on the severity tags in the `LiveLog`
    severityAdded(state, action) {
      // only add if it's not already there
      if(state.selected_severities.indexOf(action.item) != -1) {
        return {...state};
      }

      return {...state,
        selected_severities: [...state.selected_severities, action.item],
      };
    },

    /// handles clicks on the close button for a selected severity in `LiveLog`
    severityClosed(state, action) {
      let new_state = {...state,
        selected_severities: state.selected_severities.filter(severity => severity != action.item),
      };
      return new_state;
    },

    /// handles clicks on the instance tags in the `LiveLog`
    instanceAdded(state, action) {
      // only add if it's not already there
      if(state.selected_instances.filter(sender => sender.uuid == action.item.uuid).length !== 0) {
        return {...state};
      }

      return {...state,
        selected_instances: [...state.selected_instances, action.item],
      };
    },

    /// handles clicks on the close button for a selected instance tag in `LiveLog`.
    instanceClosed(state, action) {
      return {...state,
        selected_instances: state.selected_instances.filter(instance => instance.uuid != action.item.uuid),
      };
    },

    /// handles clicks on the checkboxes for setting whether the selected categores and severities should be
    /// used to filter log messages that match them or do not match them.
    toggleMatch(state, action) {
      return {...state,
        inclusive: !state.inclusive,
      };
    },
  }
}
