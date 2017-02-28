//! Backtest Management Interface

import React from 'react';

import ContentContainer from '../components/ContentContainer';
import BacktesterSpawner from '../components/backtest/BacktesterSpawner';
import BacktestStarter from '../components/backtest/BacktestStarter';
import BacktestMonitor from '../components/backtest/BacktestMonitor';

function BacktestPage() {
  return (
    <div>
      <h2>{'Spawn Backtester'}
      <BacktesterSpawner />
      <h2>{'Start Backtest'}</h2>
      <BacktestStarter />
      <h2>{'Manage Running Backtests'}
      <BacktestMonitor />
    </div>
  );
}

export default () => { return (
  <ContentContainer title="Backtest Management">
    <BacktestPage />
  </ContentContainer>
);};

