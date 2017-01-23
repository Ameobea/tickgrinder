LD_LIBRARY_PATH=../../dist/lib $(find target | rg target/debug/deps/fuzz- --no-line-number)
