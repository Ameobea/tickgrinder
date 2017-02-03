cd ../../broker_shims/FXCM/native && cargo clean && ./install.sh
rm ../../simbroker/target -rf
cd ../../../ && make dev
cd private && cargo clean && ./install.sh
cd ../scripts/fuzz && cargo clean && cargo build
