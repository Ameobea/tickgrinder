LD_LIBRARY_PATH=../dist/lib:../util/target/release/deps cargo build
cp target/debug/libprivate.so ../dist/lib
