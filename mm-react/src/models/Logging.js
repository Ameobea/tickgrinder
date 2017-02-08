//! Event handlers for actions on the logging page such as filtering messages.

export default {
  namespace: 'logging',

  state: {
    selected_categories: [],
    selected_severities: [],
  },

  reducers: {
    // handles clicks on the category tags in the `LiveLog`
    categoryAdded(state, action) {
      // only add if it's not already there
      if(state.selected_categories.indexOf(action.item) != -1) {
        return {...state};
      }
      return {...state,
        selected_categories: [...state.selected_categories, action.item],
      };
    },

    // handles clicks on the close button for selected categories in `LiveLog`
    categoryClosed(state, action) {
      return {...state,
        selected_categories: state.selected_categories.filter(category => category != action.name),
      };
    },

    // handles cliks on the severity tags in the `LiveLog`
    severityAdded(state, action) {
      // only add if it's not already there
      if(state.selected_severities.indexOf(action.item) != -1) {
        return {...state};
      }
      return {...state,
        selected_severities: [...state.selected_severities, action.item],
      };
    },

    severityClosed(state, action) {
      return {...state,
        selected_severities: state.selected_severities.filter(severity => severity != action.name),
      };
    }
  }
}
