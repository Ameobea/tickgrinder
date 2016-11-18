g++ -shared -fPIC src/libfxcm_ffi.cpp src/libfxcm_ffi.h -Wl,--no-undefined -o dist/libfxcm_ffi.so \
-Isrc -Iinclude/ForexConnectAPI-Linux-x86_64/include -Iinclude/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/Login/source \
-Iinclude/ForexConnectAPI-Linux-x86_64/samples/cpp/sample_tools/include/ include/ForexConnectAPI-Linux-x86_64/include/ForexConnect.h \
dist/libForexConnect.so dist/libfxmsg.so include/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/Login/source/SessionStatusListener.cpp \
dist/libsample_tools.so
