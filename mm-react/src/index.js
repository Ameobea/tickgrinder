import dva from 'dva';
import React from 'react';
import ReactDOM from 'react-dom';

import { LocaleProvider } from 'antd';
import enUS from 'antd/lib/locale-provider/en_US';

const GlobalState = require('./models/GlobalState');
const PlatformCommunication = require('./models/PlatformCommunication');
const Logging = require('./models/Logging');
const InstanceManagement = require('./models/InstanceManagement');
const Macros = require('./models/macros');
const Documents = require('./models/Documents');
const Data = require('./models/Data');

// 1. Initialize
const app = dva();

// 2. Plugins
// app.use({});

// 3. Model
app.model(GlobalState);
app.model(PlatformCommunication);
app.model(Logging);
app.model(InstanceManagement);
app.model(Macros);
app.model(Documents);
app.model(Data);

// 4. Router
app.router(require('./router'));

// 5. Start
const App = app.start();

ReactDOM.render(
  <LocaleProvider locale={enUS}>
    <App />
  </LocaleProvider>
, document.getElementById('root')
);
