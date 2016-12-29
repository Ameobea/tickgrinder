//! Contains all private indicators you may devise for your system.

use algobot_util::transport::postgres::*;
use algobot_util::conf::CONF;

mod sma;

pub use self::sma::Sma;
