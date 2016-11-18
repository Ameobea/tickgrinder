This is a broker shim that uses the native FXCM ForexConnect C++ API to communicate with the broker and exposes its functionality in Rust.

The code in the `native` directory creates a C++ library that is used to actually communicate with the broker (further documented in the README file inside that directory) and the `src` directory of this Cargo project contains Rust code implementing the Broker trait and communicating with the C++ API via FFI.
