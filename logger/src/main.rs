//! Logger module for the platform.  Ingests, processes, and forwards logs from all
//! of the platform's modules.

#![feature(test)]

#[macro_use]
extern crate log;
extern crate test;

fn main() {
    println!("Hello, world!");
}

#[bench]
fn name(b: &mut test::Bencher) {
    b.iter(|| {
        println!("this is printed by default")
    })
}
