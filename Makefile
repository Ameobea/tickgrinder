SHELL := /bin/bash

# All modules of the platform are dynamically linked, so rust's `libstd` is copied into the dist directory first.
# Dependencies for the platform are somewhat complicated; the Configurator must be built and run first because it
# generates the conf files that are compiled into `tickgrinder_util` and `mm`.  Then, `tickgrinder_util` must be
# built because pretty much everything else depends on it.  After that, the `private` module must be built since
# it contains code used in many of the platform's modules such as the tick processor and the optimizer.
#
# However, the FXCM shim must be built in order for the private module to work with the FXCM native broker, so
# it is built first.  That shim depends on the FXCM native broker libraries contained in a git submodule;
# those are automatically copied into `dist/lib` during the build process.
#
# All dependency libraries are copied into the `dist/lib` directory after compilation.  In addition, for all
# modules execpt for the configurator (since it is a dependency of util), the pre-built crate dependencies are
# re-used from the `util` module's `target/release/deps`; this is why `extern crate` imports are used in the
# platform's modules without any crates listed as dependencies in their `Cargo.toml` files.

release:
	make init
	make node

	# copy libstd to the dist/lib directory if it's not already there
	if [[ ! -f dist/lib/$$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) ]]; then \
		cp $$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) dist/lib; \
	fi;

	# Run the configurator if no settings exist from a previous run
	if [[ ! -f configurator/settings.json ]]; then cd configurator && cargo run; fi;

	# build the platform's utility library and copy into dist/lib
	cd util && CARGO_INCREMENTAL=1 cargo build --release
	cp util/target/release/libtickgrinder_util.so dist/lib

	# Build and install the small libboost rand wrapper
	cd private/src/strategies/fuzzer/extern && ./build.sh
	cp private/src/strategies/fuzzer/extern/librand_bindings.so dist/lib

	# build the broker shims
	cd broker_shims/simbroker && CARGO_INCREMENTAL=1 cargo build --release
	cp broker_shims/simbroker/target/release/libsimbroker.so dist/lib
	cd broker_shims/FXCM/native/native && ./build.sh
	cp broker_shims/FXCM/native/native/dist/* dist/lib
	cd broker_shims/FXCM/native && CARGO_INCREMENTAL=1 cargo build --release
	cp broker_shims/FXCM/native/target/release/libfxcm.so dist/lib

	# build the private library containing user-specific code
	cd private && CARGO_INCREMENTAL=1 cargo build --release
	cp private/target/release/libprivate.so dist/lib

	# build all modules and copy their binaries into the dist directory
	cd backtester && CARGO_INCREMENTAL=1 cargo build --release
	cp backtester/target/release/backtester dist
	cd spawner && CARGO_INCREMENTAL=1 cargo build --release
	cp spawner/target/release/spawner dist
	cd tick_parser && CARGO_INCREMENTAL=1 cargo build --release
	cp tick_parser/target/release/tick_processor dist

	# build the FXCM data downloaders
	cd data_downloaders/fxcm_native && CARGO_INCREMENTAL=1 cargo build --release
	cp data_downloaders/fxcm_native/target/release/fxcm_native dist/fxcm_native_downloader
	cd data_downloaders/fxcm_flatfile && CARGO_INCREMENTAL=1 cargo build --release
	cp data_downloaders/fxcm_flatfile/target/release/fxcm_flatfile dist/fxcm_flatfile_downloader

	cd optimizer && CARGO_INCREMENTAL=1 cargo build --release
	cp optimizer/target/release/optimizer dist
	cd logger && CARGO_INCREMENTAL=1 cargo build --release
	cp logger/target/release/logger dist

	# compile the MM
	cd mm-react && npm run build

dev:
	# rm dist/mm -r
	# cd dist && ln -s ../mm/ ./mm

	# build the simbroker in superlog mode
	cd broker_shims/simbroker && RUSTFLAGS="-L ../../util/target/release/deps -L ../../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build --features="superlog"
	cp broker_shims/simbroker/target/debug/libsimbroker.so dist/lib

dev_release:
	rm dist/mm -r
	cd dist && ln -s ../mm/ ./mm

	# build the simbroker in superlog mode
	cd broker_shims/simbroker && RUSTFLAGS="-L ../../util/target/release/deps -L ../../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build --release --features="superlog"
	cp broker_shims/simbroker/target/release/libsimbroker.so dist/lib

debug:
	make init
	make node

	# copy libstd to the dist/lib directory if it's not already there
	if [[ ! -f dist/lib/$$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) ]]; then \
		cp $$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) dist/lib; \
	fi;

	# build the configurator
	cd configurator && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build

	# Run the configurator if no settings exist from a previous run
	if [[ ! -f configurator/settings.json ]]; then cd configurator && cargo run; fi;

	# build the platform's utility library and copy into dist/lib
	cd util && CARGO_INCREMENTAL=1 cargo build
	cp util/target/debug/libtickgrinder_util.so dist/lib

	# Build and install the small libboost rand wrapper
	cd private/src/strategies/fuzzer/extern && ./build.sh
	cp private/src/strategies/fuzzer/extern/librand_bindings.so dist/lib

	# build the broker shims
	cd broker_shims/simbroker && RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp broker_shims/simbroker/target/debug/libsimbroker.so dist/lib
	cd broker_shims/FXCM/native/native && ./build.sh
	cp broker_shims/FXCM/native/native/dist/* dist/lib
	cd broker_shims/FXCM/native && RUSTFLAGS="-L ../../../util/target/debug/deps -L ../../../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp broker_shims/FXCM/native/target/debug/libfxcm.so dist/lib

	# build the private library containing user-specific code as well as the small C++ wrapper
	cd private && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp private/target/debug/libprivate.so dist/lib

	# build all modules and copy their binaries into the dist directory
	cd backtester && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp backtester/target/debug/backtester dist
	cd spawner && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp spawner/target/debug/spawner dist
	cd tick_parser && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp tick_parser/target/debug/tick_processor dist

	# build the FXCM native data downloader
	cd data_downloaders/fxcm_native && RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp data_downloaders/fxcm_native/target/debug/fxcm_native dist/fxcm_native_downloader
	cd data_downloaders/fxcm_flatfile && RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp data_downloaders/fxcm_flatfile/target/debug/fxcm_flatfile dist/fxcm_flatfile_downloader

	cd optimizer && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp optimizer/target/debug/optimizer dist
	cd logger && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp logger/target/debug/logger dist
	cd mm && npm install
	cp ./mm dist -r

strip:
	cd dist && strip backtester spawner optimizer tick_processor
	cd dist/lib && strip *

clean:
	rm optimizer/target -rf
	rm logger/target -rf
	rm spawner/target -rf
	rm tick_parser/target -rf
	rm util/target -rf
	rm backtester/target -rf
	rm mm/node_modules -rf
	rm private/target -rf
	rm broker_shims/simbroker/target -rf
	rm broker_shims/FXCM/native/native/dist -rf
	rm broker_shims/FXCM/native/target -rf
	rm data_downloaders/fxcm_native/target -rf
	rm configurator/target -rf

test:
	# copy libstd to the dist/lib directory if it's not already there
	if [[ ! -f dist/lib/$$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) ]]; then \
		cp $$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) dist/lib; \
	fi;

	cd configurator && LD_LIBRARY_PATH="../../dist/lib" RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	# build the platform's utility library and copy into dist/lib
	cd util && CARGO_INCREMENTAL=1 cargo build && cargo test --no-fail-fast
	cp util/target/debug/libtickgrinder_util.so dist/lib

	# Build and install the small libboost rand wrapper
	cd private/src/strategies/fuzzer/extern && ./build.sh
	cp private/src/strategies/fuzzer/extern/librand_bindings.so dist/lib

	# build and test the broker shims
	cd broker_shims/simbroker && LD_LIBRARY_PATH=../../util/target/debug/deps \
		RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" cargo test && \
		RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cp broker_shims/simbroker/target/debug/libsimbroker.so dist/lib
	cd broker_shims/FXCM/native/native && ./build.sh
	cp broker_shims/FXCM/native/native/dist/* dist/lib
	cd broker_shims/FXCM/native && RUSTFLAGS="-L ../../../util/target/debug/deps -L ../../../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build && \
		LD_LIBRARY_PATH=native/dist:../../../util/target/debug/deps \
		RUSTFLAGS="-L ../../../util/target/debug/deps -L ../../../dist/lib -C prefer-dynamic" cargo test -- --nocapture
	cp broker_shims/FXCM/native/target/debug/libfxcm.so dist/lib

	# build private
	cd private && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" CARGO_INCREMENTAL=1 cargo build
	cd private && LD_LIBRARY_PATH="../dist/lib" RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test --no-fail-fast

	cd optimizer && LD_LIBRARY_PATH="../dist/lib" RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	cd logger && LD_LIBRARY_PATH="../dist/lib" RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	cd spawner && LD_LIBRARY_PATH="../dist/lib" RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	cd tick_parser && LD_LIBRARY_PATH="../dist/lib" RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	cd backtester && LD_LIBRARY_PATH="../dist/lib" RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	cd mm && npm install
	cp private/target/debug/libprivate.so dist/lib
	cd data_downloaders/fxcm_native && LD_LIBRARY_PATH="../../dist/lib" RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	cd data_downloaders/fxcm_flatfile && LD_LIBRARY_PATH="../../dist/lib" RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	# TODO: Collect the results into a nice format

bench:
	make init

	# copy libstd to the dist/lib directory if it's not already there
	if [[ ! -f dist/lib/$$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) ]]; then \
		cp $$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) dist/lib; \
	fi;

	# build the platform's utility library and copy into dist/lib
	cd util && CARGO_INCREMENTAL=1 cargo build --release && cargo bench
	cp util/target/release/libtickgrinder_util.so dist/lib

	# Build and install the small libboost rand wrapper
	cd private/src/strategies/fuzzer/extern && ./build.sh
	cp private/src/strategies/fuzzer/extern/librand_bindings.so dist/lib

	# build the broker shims
	cd broker_shims/simbroker && CARGO_INCREMENTAL=1 cargo build --release && cargo bench
	cp broker_shims/simbroker/target/release/libsimbroker.so dist/lib
	cd broker_shims/FXCM/native/native && ./build.sh
	cp broker_shims/FXCM/native/native/dist/* dist/lib
	cd broker_shims/FXCM/native && CARGO_INCREMENTAL=1 cargo build --release
	cp broker_shims/FXCM/native/target/release/libfxcm.so dist/lib

	# build private
	cd private && LD_LIBRARY_PATH="../dist/lib" cargo bench

	cd optimizer && LD_LIBRARY_PATH="../dist/lib" cargo bench
	cd logger && LD_LIBRARY_PATH="../dist/lib" cargo bench
	cd spawner && LD_LIBRARY_PATH="../dist/lib" cargo bench
	cd tick_parser && LD_LIBRARY_PATH="../dist/lib" cargo bench
	cd backtester && LD_LIBRARY_PATH="../dist/lib" cargo bench
	cd mm && npm install
	cd configurator && LD_LIBRARY_PATH="../dist/lib" cargo bench
	# TODO: Collect the results into a nice format

update:
	cd optimizer && cargo update
	cd logger && cargo update
	cd spawner && cargo update

	cd tick_parser && cargo update
	cd util && cargo update
	cd backtester && cargo update
	cd private && cargo update
	cd mm && npm update
	cd broker_shims/FXCM/native && cargo update
	cd configurator && cargo update
	git submodule update

doc:
	cd configurator && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports
	cd optimizer && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports -L ../util/target/release/deps -L ../dist/lib
	cd logger && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports -L ../util/target/release/deps -L ../dist/lib
	cd spawner && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports -L ../util/target/release/deps -L ../dist/lib

	cd tick_parser && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports -L ../util/target/release/deps -L ../dist/lib
	cd util && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports -L ../util/target/release/deps -L ../dist/lib
	cd backtester && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports -L ../util/target/release/deps -L ../dist/lib
	cd private && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports -L ../util/target/release/deps -L ../dist/lib
	cd mm && npm install
	# TODO: Collect the results into a nice format

# kill off any straggler processes
kill:
	if [[ $$(ps -aux | grep '[t]arget/debug') ]]; then \
		kill $$(ps -aux | grep '[t]arget/debug' | awk '{print $$2}'); \
	fi
	if [[ $$(ps -aux | grep '[m]anager.js') ]]; then \
		kill $$(ps -aux | grep '[m]anager.js' | awk '{print $$2}'); \
	fi

configure:
	cd configurator && cargo run
	cp configurator/conf.rs util/src
	cp configurator/conf.js mm-react/src

config:
	make configure

init:
	git submodule update --init
	rm -rf dist
	mkdir dist
	mkdir dist/lib

node:
	if [[ ! $$(which dva) ]]; then npm install -g dva-cli; fi
	if [[ ! -f ./mm-react/node_modules/installed ]]; then \
		cd mm-react && npm install react && npm install react-dom && npm install babel-plugin-import --save && npm install && \
			npm install dva-loading --save && touch ./node_modules/installed; \
	fi

	# fetch a built copy of the ckeditor.  If ameo.link is dead and gone, you can build your own version
	curl https://ameo.link/u/422.tgz -o mm-react/public/ckeditor.tgz
	cd mm-react/public && \
		tar -xzf ckeditor.tgz && \
		rm ckeditor.tgz && \
		cd ckeditor
