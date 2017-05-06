#!/bin/bash

# Initializes a new strategy directory with dynamic dependencies and a pre-built 

if [ $# -eq 0 ]
  then
    echo -e "\e[31musage: ./new.sh strategy_name\e[0m"
    exit 1
fi

cargo init $1
mkdir $1/.cargo
cp .templates/config $1/.cargo/config
cp sample/src/lib.rs $1/src/lib.rs

cat $1/Cargo.toml | sed "s/\[dependencies\]/[lib]\n\
name = \"$1\"\n\
crate-type = [\"dylib\"]/" > $1/Cargo.toml.tmp

mv $1/Cargo.toml.tmp $1/Cargo.toml
