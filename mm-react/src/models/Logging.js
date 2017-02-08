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
      return {...state,
        selected_categories: [...state.selected_categories, action.item],
      };
    },

    // handles cliks on the severity tags in the `LiveLog`
    severityAdded(state, action) {
      return {...state,
        selected_severities: [...state.selected_severities, action.item],
      };
    }
  }
}
