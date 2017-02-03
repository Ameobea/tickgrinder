LD_LIBRARY_PATH=../dist/lib:../util/target/release/deps cargo build --release
cp target/release/libprivate.so ../dist/lib
