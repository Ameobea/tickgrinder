export default {
  namespace: 'instances',

  state: {
    living_instances: {},
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
