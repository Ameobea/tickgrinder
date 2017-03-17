#!/bin/bash

# This script starts the whole platform and is the only official way to start the platform.
# It initializes a spawner instance which in turn spawns a Logger and a MM instance.
# Once you run this script, you can view the MM web console in your web browser (default port 8002).

LD_LIBRARY_PATH="$(pwd)/dist/lib"
export LD_LIBRARY_PATH
cd dist && RUST_BACKTRACE=1 RUST_BACKTRACE=1 ./spawner
