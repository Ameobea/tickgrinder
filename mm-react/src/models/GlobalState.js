const initialState = {
  title: "Default Title",
  content_function: () => "Default Content",
}

export default {
  namespace: 'global',

  state: {
    title: "Original Title",
  },

  reducers: {
    pageChange(state = initialState, action) {
      state.title = action.title;
      return state;
    }
  },

  effects: {

  },

  subscriptions: {
    CommandListener({ todo }) {
      // TODO
    }
  }
}
