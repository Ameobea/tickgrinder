rm dist -r
mkdir -p dist
cp -u include/ForexConnectAPI-Linux-x86_64/lib/*.so dist
cp include/ForexConnectAPI-Linux-x86_64/samples/cpp/sample_tools/lib/libsample_tools.so dist
g++ -shared -fPIC src/libfxcm_ffi.cpp src/libfxcm_ffi.h include/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/GetHistPrices/source/CommonSources.cpp \
include/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/GetHistPrices/source/ResponseListener.cpp -Wl,--no-undefined -o dist/libfxcm_ffi.so \
src/offers.cpp include/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/GetHistPrices/source/LoginParams.cpp \
-Isrc -Iinclude/ForexConnectAPI-Linux-x86_64/include -Iinclude/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/Login/source \
-Iinclude/ForexConnectAPI-Linux-x86_64/samples/cpp/sample_tools/include/ include/ForexConnectAPI-Linux-x86_64/include/ForexConnect.h \
-Iinclude/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/GetHistPrices/source/ \
dist/libForexConnect.so dist/libfxmsg.so include/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/Login/source/SessionStatusListener.cpp \
dist/libsample_tools.so
