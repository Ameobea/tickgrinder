import React from 'react';
import { Router, Route } from 'dva/router';

import AppPage from './components/AppPage';
import IndexPage from './routes/IndexPage';
import Backtest from './routes/Backtest';
import Logging from './routes/Logging';
import DataManagement from './routes/DataManagement';
import InstanceManagement from './routes/InstanceManagement';

function RouterConfig({history}) {
  return ( // TODO: notification counts for tabs
      <Router history={history}>
          <Route component={AppPage} path="/">
              <Route component={IndexPage} path="/index" />
              <Route component={Backtest} path="/backtest" />
              <Route component={DataManagement} path="/data" />
              <Route component={Logging} path="/log" />
              <Route component={InstanceManagement} path="/instances" />
          </Route>
      </Router>
  );
}

export default RouterConfig;
