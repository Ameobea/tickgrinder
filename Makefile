build:
	cd optimizer && cargo build
	cd spawner && cargo build

	# cd strategies
	# # Build each strategy
	# for dir in ./strategies/*/; \
	# do \
	# 	cd $$dir && cargo build; \
	# done

	cd tick_parser && cargo build
	cd util && cargo build
	# TODO: Collect the results into a nice format

release:
	cd optimizer && cargo build --release
	cd spawner && cargo build --release

	# for dir in ./strategies/*/; \
	# do \
	# 	cd $$dir && cargo build --release; \
	# done

	cd tick_parser && cargo build --release
	cd util && cargo build --release
	# TODO: Collect the results into a nice format

clean:
	rm optimizer/target -rf
	rm spawner/target -rf

	for dir in ./strategies/*/; \
	do \
		rm $$dir/target -rf; \
	done

	rm tick_parser/target -rf
	rm util/target -rf
	rm dist -rf

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
	# TODO: Collect the results into a nice format

bench:
	cd tick_parser && cargo bench
	cd util && cargo bench
	for dir in ./strategies/*/; \
	do \
		cd $$dir && cargo bench; \
	done
	# TODO: Collect the results into a nice format

install:
	mkdir -p dist
	cp optimizer/target/release/optimizer dist
	cp ./mm dist -r
	cp spawner/target/release/spawner dist
	cp tick_parser/target/release/tick_processor dist
