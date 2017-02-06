export default {
  namespace: 'global',

  state: {
    title: "TickGrinder Dashboard"
  },

  reducers: {
    save(state, action) {
      return { ...state, ...action.payload };
    },
  },

  subscriptions: {
    CommandListener({ todo }) {
      // TODO
    }
  }
}
