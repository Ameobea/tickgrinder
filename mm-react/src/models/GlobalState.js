//! Holds global state for the application; stuff like title.

import { v4 } from '../utils/commands';

export default {
  namespace: 'global',

  state: {
    title: 'Original Title',
    uuid: v4()
  },

  reducers: {
    pageChange (state, action) {
      state.title = action.title;
      return state;
    }
  },

  effects: { },

  subscriptions: {
    CommandListener ({ todo }) {
      // TODO
    }
  }
};
