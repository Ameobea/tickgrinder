valgrind --tool=callgrind $(find target | rg target/debug/deps/fuzz- --no-line-number)
