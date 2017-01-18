rm dist -r
mkdir -p dist
cp -u include/ForexConnectAPI-Linux-x86_64/lib/*.so dist
cp include/ForexConnectAPI-Linux-x86_64/samples/cpp/sample_tools/lib/libsample_tools.so dist
g++ -g -rdynamic -shared -fPIC -std=c++11 -O3 src/libfxcm_ffi.cpp src/broker_server.cpp src/GlobalResponseListener.cpp \
include/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/GetHistPrices/source/CommonSources.cpp \
-I include/ForexConnectAPI-Linux-x86_64/include/ -I include/ForexConnectAPI-Linux-x86_64/samples/cpp/TableManagerSamples/GetOffers/source/ \
-lboost_system -lboost_thread include/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/GetHistPrices/source/ResponseListener.cpp \
-Wl,--no-undefined -o dist/libfxcm_ffi.so src/GlobalTableListener.cpp src/SessionStatusListener.cpp src/trade_execution.cpp \
src/offers.cpp include/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/GetHistPrices/source/LoginParams.cpp \
-Iinclude/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/Login/source \
-Iinclude/ForexConnectAPI-Linux-x86_64/samples/cpp/sample_tools/include/ include/ForexConnectAPI-Linux-x86_64/include/ForexConnect.h \
-Iinclude/ForexConnectAPI-Linux-x86_64/samples/cpp/NonTableManagerSamples/GetHistPrices/source/ \
dist/libForexConnect.so dist/libfxmsg.so dist/libsample_tools.so
cp dist/libfxcm_ffi.so ../../../../../../../../dist/lib
