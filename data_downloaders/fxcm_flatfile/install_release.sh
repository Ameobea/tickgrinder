CARGO_INCREMENTAL=1 LD_LIBRARY_PATH=../dist/lib:../util/target/release/deps cargo build --release
cp target/release/fxcm_flatfile ../dist/fxcm_flatfile_downloader
