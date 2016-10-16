//! Defines a backtest, which determines what data is sent and the
//! conditions that trigger it to be sent.

pub trait Backtest {
    fn init() -> Self;
}
