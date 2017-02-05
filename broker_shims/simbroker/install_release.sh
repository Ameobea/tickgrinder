CARGO_INCREMENTAL=1 cargo build --release
cp target/debug/libsimbroker.so ../../dist/lib
