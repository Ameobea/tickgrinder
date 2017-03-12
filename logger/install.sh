CARGO_INCREMENTAL=1 LD_LIBRARY_PATH=../dist/lib:../util/target/release/deps cargo build
cp target/debug/logger ../dist/
