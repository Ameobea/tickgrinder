//! Trading conditions are expressions that the Tick Processor evaluates for every received tick.
//! If the condition returns a TradingAction when evaluated, that action is executed.

use uuid::Uuid;

use trading::tick::Tick;

pub trait TradingCondition {
    fn eval(&self, t: Tick) -> Option<TradingAction>;

    fn to_string(&self) -> String;

    fn from_string(s: String) -> Self;
}

#[derive(Clone, Debug)]
pub enum TradingAction {
    /// Opens an order at market price +-max_range pips.
    MarketOrder {
        account: Uuid, symbol: String, long: bool, size: usize, stop: Option<usize>,
        take_profit: Option<usize>, max_range: Option<f64>
    },
    /// Opens an order at a price equal or better to `entry_price` as soon as possible.
    LimitOrder{
        account: Uuid, symbol: String, long: bool, size: usize, stop: Option<usize>,
        take_profit: Option<usize>, entry_price: usize
    },
    /// Closes `size` lots of a position with the specified UUID.
    ClosePosition{ uuid: Uuid },
    /// Modifies a position without taking any trading action.
    ModifyPosition{
        uuid: Uuid, stop: Option<usize>, take_profit: Option<usize>, entry_price: Option<usize>
    },
}
