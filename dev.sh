#!/bin/bash

# This script symlinks the mm directory into the dist directory so that changes made to the
# mm directory during development will be reflected when running the platform.
#
# You only need to run this script if you're working on the MM.

make debug && make dev && ./run.sh
