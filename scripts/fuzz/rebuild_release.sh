cd ../../broker_shims/FXCM/native && cargo clean && ./install_release.sh
rm ../../simbroker/target -rf
cd ../../../ && make dev_release
cd private && cargo clean && ./install_release.sh
cd ../scripts/fuzz && cargo clean && cargo build --release
