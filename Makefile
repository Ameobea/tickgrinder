SHELL := /bin/bash

build:
	rm -rf dist
	mkdir dist
	mkdir dist/lib
	cd util && cargo rustc -- -C prefer-dynamic
	cp util/target/debug/libalgobot_util.so dist/lib

	# build all strategies and copy into dist/lib
	for dir in ./strategies/*; \
	do \
		cd $$dir && cargo rustc -- -C prefer-dynamic -L ../../util/target/debug/deps -L ../../dist/lib && \
		cp target/debug/lib$$(echo $$dir | sed "s/\.\/strategies\///").so ../../dist/lib; \
	done

	cd backtester && cargo rustc --bin backtester -- -C prefer-dynamic -L ../util/target/debug/deps -L ../dist/lib
	cp backtester/target/debug/backtester dist
	cd spawner && cargo rustc --bin spawner -- -C prefer-dynamic -L ../util/target/debug/deps -L ../dist/lib
	cp spawner/target/debug/spawner dist
	cd tick_parser && cargo rustc --bin tick_processor -- -C prefer-dynamic -L ../util/target/debug/deps -L ../dist/lib
	cp tick_parser/target/debug/tick_processor dist
	cd optimizer && cargo rustc --bin optimizer -- -C prefer-dynamic -L ../util/target/debug/deps -L ../dist/lib
	cp optimizer/target/debug/optimizer dist
	cd mm && npm install
	cp ./mm dist -r

release:
	rm -rf dist
	mkdir dist
	mkdir dist/lib
	cd util && cargo rustc --release -- -C prefer-dynamic
	cp util/target/release/libalgobot_util.so dist/lib

	# build all strategies and copy into dist/lib
	for dir in ./strategies/*; \
	do \
		cd $$dir && cargo rustc --release -- -C prefer-dynamic -L ../../util/target/release/deps -L ../../dist/lib && \
		cp target/release/lib$$(echo $$dir | sed "s/\.\/strategies\///").so ../../dist/lib; \
	done

	cd backtester && cargo rustc --bin backtester --release -- -C prefer-dynamic -L ../util/target/release/deps -L ../dist/lib
	cp backtester/target/release/backtester dist
	cd spawner && cargo rustc --bin spawner --release -- -C prefer-dynamic -L ../util/target/release/deps -L ../dist/lib
	cp spawner/target/release/spawner dist
	cd tick_parser && cargo rustc --bin tick_processor --release -- -C prefer-dynamic -L ../util/target/release/deps -L ../dist/lib
	cp tick_parser/target/release/tick_processor dist
	cd optimizer && cargo rustc --bin optimizer --release -- -C prefer-dynamic -L ../util/target/release/deps -L ../dist/lib
	cp optimizer/target/release/optimizer dist
	cd mm && npm install
	cp ./mm dist -r

strip:
	strip dist/*

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

test:
	cd optimizer && cargo test
	cd spawner && cargo test

	# Build each strategy
	for dir in ./strategies/*/; \
	do \
		cd $$dir && cargo test; \
	done

	cd tick_parser && cargo test
	cd util && cargo test
	cd backtester && cargo test
	cd mm && npm install
	# TODO: Collect the results into a nice format

bench:
	cd tick_parser && cargo bench
	cd util && cargo bench
	cd backtester && cargo bench
	for dir in ./strategies/*/; \
	do \
		cd $$dir && cargo bench; \
	done
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
