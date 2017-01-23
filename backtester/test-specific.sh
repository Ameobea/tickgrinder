LD_LIBRARY_PATH=../dist/lib RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test retrans -- --nocapture
