//! Conditions that are evaluated for every new Tick received by a Tick Processor.  They return
//! `TradingAction`s that are used to actually execute trades, modify positions, etc.

pub mod sma_cross;

use tickgrinder_util::trading::trading_condition::*;

// use indicators::*;
use self::sma_cross::*;

/// Contains every indicator that you may want to use in your platform.
pub enum TradingConditions {
    SmaCross{period: usize},
}

impl TradingConditions {
    fn get(&self) -> impl TradingCondition {
        match *self {
            TradingConditions::SmaCross{period} => SmaCross::new(period)
        }
    }
}
