FXCM Native Data Downloader

This data downloader makes use of the FXCM native API (located in libfxcm_ffi.so with source code in the broker shims directory of the util library) to enable data downloading from FXCM without requiring a remote Java client.

The remote API works by passing a callback function to the C++ code which is run for each downloaded tick.  The C++ downloader code is responsible for downloading all data in the specified range and calling the callback for each individual tick received.  There is no guarentee that the received ticks will be in order, only that they will all be received.
