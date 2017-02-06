import dva from 'dva';

// 1. Initialize
const app = dva();

// 2. Plugins
// app.use({});

// 3. Model
// app.model(require('./models/example'));
app.model(require('./models/instances'));

// 4. Router
app.router(require('./router'));

// 5. Start
app.start("#root");
