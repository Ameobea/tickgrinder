import dva from 'dva';
import React from 'react';
import ReactDOM from 'react-dom';

import { LocaleProvider } from 'antd';
import enUS from 'antd/lib/locale-provider/en_US';

import IndexPage from './routes/IndexPage';
let GlobalState = require('./models/GlobalState');
let PlatformCommunication = require('./models/PlatformCommunication');
let Logging = require('./models/Logging');

// 1. Initialize
const app = dva();

// 2. Plugins
// app.use({});

// 3. Model
// app.model(require('./models/example'));
app.model(GlobalState);
app.model(PlatformCommunication);
app.model(Logging);

// 4. Router
app.router(require('./router'));
// app.router(() => <App />)

// 5. Start
const App = app.start();

ReactDOM.render(
  <LocaleProvider locale={enUS}>
    <App />
  </LocaleProvider>
, document.getElementById('root')
);
