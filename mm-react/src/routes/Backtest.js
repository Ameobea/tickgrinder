//! Backtest Management Interface

import React from 'react';

import ContentContainer from '../components/ContentContainer';

function BacktestPage() {
  return (
    <div>Backtests</div>
  );
}

export default () => <ContentContainer title="Backtest Management" content_function={BacktestPage} />;
