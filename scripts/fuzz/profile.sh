LD_LIBRARY_PATH=../../dist/lib valgrind --tool=callgrind target/release/fuzz --dump-instr=yes
