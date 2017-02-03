CARGO_INCREMENTAL=1 LD_LIBRARY_PATH=../../dist/lib $(find target | rg target/release/deps/fuzz- --no-line-number)
