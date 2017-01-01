//! The broker server is a layer implemented in C++ that allows for direct interaction with the
//! FXCM native API without worrying about passing function pointers around, maintaining C++
//! state rust-side, etc.
//!
//! It returns some function pointers to Rust that, combined with a pointer to the internal
//! environment, can be used to send commands to the server.  It sends resposnes to a Rust
//! channel by calling the exported `send()` function on the C++ side.

#include "stdafx.h"
#include "CommonSources.h"
#include "libfxcm_ffi.h"

#include <boost/lockfree/spsc_queue.hpp>

/// Contains the inner server state that can be sent along with commands to be processed
struct Environment {
    IO2GSession* session;
};

void* init_broker_server(void* void_session, void (*cb)(ServerResponse res, void* payload)) {

}

void exec_command(ServerCommand command, void* args, void* void_env) {
    Environment* env = (Environment*)void_env;
}
