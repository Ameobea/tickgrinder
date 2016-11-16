#!/bin/bash

LD_LIBRARY_PATH="$(pwd)/dist/lib"
export LD_LIBRARY_PATH
cd dist && ./spawner
