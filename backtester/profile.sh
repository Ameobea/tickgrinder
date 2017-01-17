LD_LIBRARY_PATH=/home/casey/.multirust/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/:native/dist:../util/target/release/deps \
valgrind --dump-instr=yes --tool=callgrind $(find | rg -N release/deps/backtester-) retran
