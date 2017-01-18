g++ -g -rdynamic -shared -fPIC -std=c++11 -O3 bindings.cpp -lboost_random -o librand_bindings.so -Wl,--no-undefined
