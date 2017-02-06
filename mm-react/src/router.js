import React from 'react';
import { Router, Route, browserHistory } from 'dva/router';

import AppPage from './components/AppPage';
import IndexPage from './routes/IndexPage';
import Backtest from './routes/Backtest.js';

function RouterConfig({history}) {
  return (
    <Router history={history}>
      <Route path="/" component={AppPage}>
        <Route path="/index" component={IndexPage} />
        <Route path="/backtest" component={Backtest} />
      </Route>
    </Router>
  );
}

export default RouterConfig;
