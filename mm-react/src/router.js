import React from 'react';
import { Router, Route } from 'dva/router';

import AppPage from './components/AppPage';
import IndexPage from './routes/IndexPage';
import Backtest from './routes/Backtest.js';

function RouterConfig({ history }) {
  return (
    <Router history={history}>
      <Route path="/" component={AppPage}>
        <Route path="/index" component={IndexPage} title="TickGrinder Dashboard Homepage" />
        <Route path="/backtest" component={Backtest} title="Backtest Management" />
      </Route>
    </Router>
  );
}

export default RouterConfig;
