//! The broker server is a layer implemented in C++ that allows for direct interaction with the
//! FXCM native API without worrying about passing function pointers around, maintaining C++
//! state rust-side, etc.
//!
//! It returns some function pointers to Rust that, combined with a pointer to the internal
//! environment, can be used to send commands to the server.  It sends resposnes to a Rust
//! channel by calling the exported `send()` function on the C++ side.

#include <mutex>
#include <condition_variable>

#include "stdafx.h"
#include "CommonSources.h"
#include "libfxcm_ffi.h"

#include <boost/thread/thread.hpp>
#include <boost/lockfree/spsc_queue.hpp>

typedef void (*ResponseCallback)(void* tx_ptr, ServerMessage* res);
typedef void (*QueuePush)(ClientMessage message);
typedef boost::lockfree::spsc_queue<ClientMessage, boost::lockfree::capacity<1024> > MessageQueue;

/// Contains pointers to a bunch of heap-allocated interal variables that are used by the server
/// to maintain state, provide synchronization, and store messages.
struct Environment {
    ResponseCallback cb;
    void* tx_ptr;
    MessageQueue* q;
    std::condition_variable* cond_var;
    std::mutex* m;
};

/// Puts a ClientMessage into the queue.
void push_client_message(ClientMessage msg, void* void_env) {
    Environment* env = (Environment*)void_env;
    // insert the message into the queue
    std::unique_lock<std::mutex> lock(*env->m);
    env->q->push(msg);
    lock.unlock();
    env->cond_var->notify_all();
}

/// Processes a message from the client and returns a message to be sent back or NULL.
ServerMessage* process_client_message(ClientMessage* message) {
    std::cout << "Processed message of type " << message->command << std::endl;

    return NULL;
}


/// Initializes the internal server event loop and starts listening for messages from the client.
void start_server(void* void_session, void* void_env) {
    Environment* env = (Environment*)void_env;
    IO2GSession* session = (IO2GSession*)void_session;
    ClientMessage message;
    ServerMessage* response = new ServerMessage;

    while(true) {
        std::unique_lock<std::mutex> lock(*env->m);
        env->cond_var->wait(lock, [env, &message](){ return env->q->pop(message); });

        lock.unlock();

        response = process_client_message(&message);
        // send the response asynchronously back to the client if there is one to send.
        if(response != NULL)
            env->cb(env->tx_ptr, response);
    }
}

/// Starts the internal server event loop and returns a reference to the queue that can be used along
/// with the `push_client_message()` function to send messages to it.
void* init_server_environment(ResponseCallback cb, void* tx_ptr) {
    // heap allocate all the internals so they don't go out of scope and die
    MessageQueue* q = new MessageQueue();
    std::condition_variable* cond_var = new std::condition_variable();
    std::mutex* m = new std::mutex;
    Environment* heap_env = new Environment({ cb, tx_ptr, q, cond_var, m });

    return (void*)heap_env;
}
