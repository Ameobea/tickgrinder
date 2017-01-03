#cd native && ./build.sh
# LD_LIBRARY_PATH=native/dist strace -f -e trace=network -s 10000 cargo test
LD_LIBRARY_PATH=native/dist:../../../../../../target/release/deps cargo bench -- --nocapture
