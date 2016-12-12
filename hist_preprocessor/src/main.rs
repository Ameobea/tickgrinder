//! Historical Data Preprocessors
//!
//! Apply transformations to historical data and store the results in the database.
//! See README.txt for more information.

extern crate postgres;
extern crate algobot_util;

mod conf;
mod preprocessor;
mod preprocessors;

fn main() {
    println!("Hello, world!");
}
