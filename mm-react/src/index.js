import dva from 'dva';
import { LocaleProvider } from 'antd';
import enUS from 'antd/lib/locale-provider/en_US';
import React from 'react';
import ReactDOM from 'react-dom';

import IndexPage from './routes/IndexPage';

// 1. Initialize
const app = dva();

// 2. Plugins
// app.use({});

// 3. Model
// app.model(require('./models/example'));
app.model(require('./models/GlobalState'));

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
