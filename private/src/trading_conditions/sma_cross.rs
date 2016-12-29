//! Basic trading condition that checks whether the price crosses a SMA upwards or downwards
//! and creates orders when it does.

use algobot_util::trading::tick::*;
use algobot_util::trading::trading_condition::*;

pub struct SmaCross {
    period: usize,
}

impl TradingCondition for SmaCross {
    fn eval(&mut self, t: &Tick) -> Option<TradingAction> {
        unimplemented!();
    }
}

impl SmaCross {
    pub fn new(period: usize) -> SmaCross {
        SmaCross {
            period: period,
        }
    }
}
