SHELL := /bin/bash

release:
	make init
	git submodule update --init
	rm -rf dist
	mkdir dist
	mkdir dist/lib
	# build the bot's utility library and copy into dist/lib
	cd util && cargo build --release
	cp util/target/release/libalgobot_util.so dist/lib
	# copy libstd to the dist/lib directory
	cp $$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) dist/lib

	# build all strategies and copy into dist/lib
	for dir in ./strategies/*; \
	do \
		cd $$dir && cargo build --release && \
		cp target/release/lib$$(echo $$dir | sed "s/\.\/strategies\///").so ../../dist/lib && \
		cp target/release/lib$$(echo $$dir | sed "s/\.\/strategies\///").so ../../util/target/release/deps; \
	done

	# build all modules and copy their binaries into the dist directory
	cd backtester && cargo build --release
	cp backtester/target/release/backtester dist
	cd spawner && cargo build --release
	cp spawner/target/release/spawner dist
	cd tick_parser && cargo build --release
	cp tick_parser/target/release/tick_processor dist
	cd optimizer && cargo build --release
	cp optimizer/target/release/optimizer dist
	cd mm && npm install
	cp ./mm dist -r

	# build the FXCM shim
	cd util/src/trading/broker/shims/FXCM/native/native && ./build.sh
	cp util/src/trading/broker/shims/FXCM/native/native/dist/* dist/lib
	cd util/src/trading/broker/shims/FXCM/native && cargo build --release
	cp util/src/trading/broker/shims/FXCM/native/target/release/libfxcm.so dist/lib

dev:
	rm dist/mm -r
	cd dist && ln -s ../mm/ ./mm

debug:
	make init
	git submodule update --init
	rm -rf dist
	mkdir dist
	mkdir dist/lib
	# build the bot's utility library and copy into dist/lib
	cd util && cargo build
	cp util/target/debug/libalgobot_util.so dist/lib
	# copy libstd to the dist/lib directory
	cp $$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) dist/lib

	# build all strategies and copy into dist/lib
	for dir in ./strategies/*; \
	do \
		cd $$dir && RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" cargo build && \
		cp target/debug/lib$$(echo $$dir | sed "s/\.\/strategies\///").so ../../dist/lib && \
		cp target/debug/lib$$(echo $$dir | sed "s/\.\/strategies\///").so ../../util/target/debug/deps; \
	done

	# build all modules and copy their binaries into the dist directory
	cd backtester && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo build
	cp backtester/target/debug/backtester dist
	cd spawner && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo build
	cp spawner/target/debug/spawner dist
	cd tick_parser && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo build
	cp tick_parser/target/debug/tick_processor dist
	cd optimizer && RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo build
	cp optimizer/target/debug/optimizer dist
	cd mm && npm install
	cp ./mm dist -r

	# build the FXCM shim
	cd util/src/trading/broker/shims/FXCM/native/native ./build.sh
	cp util/src/trading/broker/shims/FXCM/native/native/dist/* dist/lib
	cd util/src/trading/broker/shims/FXCM/native && RUSTFLAGS="-L ../../../../../../../util/target/debug/deps -L ../../../../../../../dist/lib -C prefer-dynamic" cargo build
	cp util/src/trading/broker/shims/FXCM/native/target/debug/libfxcm.so dist/lib

strip:
	cd dist && strip backtester spawner optimizer tick_processor
	cd dist/lib && strip *

clean:
	rm optimizer/target -rf
	rm spawner/target -rf

	for dir in ./strategies/*/; \
	do \
		rm $$dir/target -rf; \
	done

	rm tick_parser/target -rf
	rm util/target -rf
	rm backtester/target -rf
	rm mm/node_modules -rf
	rm util/src/trading/broker/shims/FXCM/native/native/dist -rf

test:
	make init
	# build the bot's utility library and copy into dist/lib
	cd util && cargo build && cargo test --no-fail-fast
	cp util/target/debug/libalgobot_util.so dist/lib
	# copy libstd to the dist/lib directory
	cp $$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) dist/lib

	# build all strategies and copy into dist/lib
	for dir in ./strategies/*; \
	do \
		cd $$dir && RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" cargo build && \
		cp target/debug/lib$$(echo $$dir | sed "s/\.\/strategies\///").so ../../dist/lib && \
		cp target/debug/lib$$(echo $$dir | sed "s/\.\/strategies\///").so ../../util/target/debug/deps && \
		LD_LIBRARY_PATH="../../dist/lib" RUSTFLAGS="-L ../../util/target/debug/deps -L ../../dist/lib -C prefer-dynamic" cargo test --no-fail-fast; \
	done

	cd optimizer && LD_LIBRARY_PATH="../dist/lib" RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	cd spawner && LD_LIBRARY_PATH="../dist/lib" RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test --no-fail-fast

	cd tick_parser && LD_LIBRARY_PATH="../dist/lib" RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	cd backtester && LD_LIBRARY_PATH="../dist/lib" RUSTFLAGS="-L ../util/target/debug/deps -L ../dist/lib -C prefer-dynamic" cargo test --no-fail-fast
	cd mm && npm install
	cd util/src/trading/broker/shims/FXCM/native && ./test.sh
	# TODO: Collect the results into a nice format

bench:
	make init
	# build the bot's utility library and copy into dist/lib
	cd util && cargo build --release && cargo bench
	cp util/target/release/libalgobot_util.so dist/lib
	# copy libstd to the dist/lib directory
	cp $$(find $$(rustc --print sysroot)/lib | grep -E "libstd-.*\.so" | head -1) dist/lib

	# build all strategies and copy into dist/lib
	for dir in ./strategies/*; \
	do \
		cd $$dir && cargo build --release && \
		cp target/release/lib$$(echo $$dir | sed "s/\.\/strategies\///").so ../../dist/lib && \
		cp target/release/lib$$(echo $$dir | sed "s/\.\/strategies\///").so ../../util/target/release/deps && \
		LD_LIBRARY_PATH="../../dist/lib" cargo bench; \
	done

	cd optimizer && LD_LIBRARY_PATH="../dist/lib" cargo bench
	cd spawner && LD_LIBRARY_PATH="../dist/lib" cargo bench

	cd tick_parser && LD_LIBRARY_PATH="../dist/lib" cargo bench
	cd backtester && LD_LIBRARY_PATH="../dist/lib" cargo bench
	cd mm && npm install
	# TODO: Collect the results into a nice format

update:
	cd optimizer && cargo update
	cd spawner && cargo update

	# Build each strategy
	for dir in ./strategies/*/; \
	do \
		cd $$dir && cargo update; \
	done

	cd tick_parser && cargo update
	cd util && cargo update
	cd backtester && cargo update
	cd mm && npm update
	cd util/src/trading/broker/shims/FXCM/native && cargo update
	git submodule update

cdoc:
	cd optimizer && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports
	cd spawner && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports

	# Build each strategy
	for dir in ./strategies/*/; \
	do \
		cd $$dir && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports; \
	done

	cd tick_parser && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports
	cd util && cargo test
	cd backtester && cargo rustdoc --open -- --no-defaults --passes collapse-docs --passes unindent-comments --passes strip-priv-imports
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

# used to set up the development environment
init:
	if [[ $$(which rustup) ]]; then \
		if [[ $$(rustup show | grep nightly-2016-10-10 ) ]]; then \
			rustup override set nightly-2016-10-10; \
		else \
			rustup install nightly-2016-10-10; \
		fi \
	else \
		echo -e "\e[91mRustup is not installed or not in the path;"; \
		echo -e "You need rustup to build this application."; \
		echo -e "\e[94mYou can install rustup here: https://rustup.rs/\e[0m"; \
		echo "Would you like to install it now? "; \
		read -p "[Y/n]:" bool; \
		if [[ $$bool =~ ^([yY][eE][sS]|[yY])$$ ]]; then \
			curl https://sh.rustup.rs -sSf | sh; \
		fi \
	fi

	if [[ ! -a tick_parser/src/conf.rs ]]; then \
		cp tick_parser/src/conf.default.rs tick_parser/src/conf.rs; \
		cp spawner/src/conf.default.rs spawner/src/conf.rs; \
		cp optimizer/src/conf.sample.rs optimizer/src/conf.rs; \
		cp mm/sources/conf.sample.js mm/sources/conf.js; \
		cp backtester/src/conf.default.rs backtester/src/conf.rs; \
		cp util/src/trading/broker/shims/FXCM/native/src/conf.default.rs util/src/trading/broker/shims/FXCM/native/src/conf.rs; \
	fi
