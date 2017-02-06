import React from 'react';
import { Router, Route, browserHistory } from 'dva/router';

import AppPage from './components/AppPage';
import IndexPage from './routes/IndexPage';
import Backtest from './routes/Backtest';
import Logging from './routes/Logging';
import DataManagement from './routes/DataManagement';

function RouterConfig({history}) {
  return (
    <Router history={history}>
      <Route path="/" component={AppPage}>
        <Route path="/index" component={IndexPage} />
        <Route path="/backtest" component={Backtest} />
        <Route path="/data" component={DataManagement} />
        <Route path="/log" component={Logging} />
      </Route>
    </Router>
  );
}

export default RouterConfig;
