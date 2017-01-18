#include <stdint.h>
#include <boost/random.hpp>

/// Creates a new deterministic psuedorandom number generator given the provided seed and returns a reference to it.
extern "C" void* init_rng(unsigned int seed) {
    boost::random::mt19937* gen = new boost::random::mt19937{seed};
    return (void*)gen;
}

/// Given a reference to a random number generator, returns a random integer from within the range [min, max].
extern "C" int rand_int_range(void* void_rng, int min, int max) {
    boost::random::mt19937* gen = (boost::random::mt19937*)void_rng;
    boost::random::uniform_int_distribution<> dist{min, max};
    return dist(*gen);
}
