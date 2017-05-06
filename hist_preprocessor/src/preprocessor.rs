//! Defines the preprocessor trait which is basically a transformation applied to raw tick
//! data that is stored in the database.

use algobot_util::trading::tick::*;

pub trait Preprocessor {
    fn process(&mut self, t: Tick);
}
