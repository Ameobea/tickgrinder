//! The broker server is a layer implemented in C++ that allows for direct interaction with the
//! FXCM native API without worrying about passing function pointers around, maintaining C++
//! state rust-side, etc.
//!
//! It returns some function pointers to Rust that, combined with a pointer to the internal
//! environment, can be used to send commands to the server.  It sends resposnes to a Rust
//! channel by calling the exported `send()` function on the C++ side.

#include <mutex>
#include <condition_variable>
#include <ctime>

#include <boost/thread/thread.hpp>
#include <boost/lockfree/spsc_queue.hpp>
#include <boost/date_time/posix_time/posix_time.hpp>

#include "stdafx.h"
#include "CommonSources.h"
#include "libfxcm_ffi.h"

typedef void (*QueuePush)(ClientMessage message);

using namespace boost::posix_time;
using namespace boost::gregorian;

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
void process_client_message(ClientMessage* message, ServerMessage* response, IO2GSession* session, Environment* env) {
    switch(message->command) {
        case PING: {
            long* heap_micros = (long*)malloc(sizeof(long));
            long stack_micros = current_timestamp_micros();
            *heap_micros = stack_micros;
            *response = ServerMessage({PONG, (void*)heap_micros});
            break;
        }
        case INIT_TICK_SUB: {
            TickstreamDef* def = (TickstreamDef*)message->payload;
            init_tick_stream(def->env_ptr, def->cb, session, env);
            *response = ServerMessage({TICK_SUB_SUCCESSFUL, NULL});
            break;
        }
        case LIST_ACCOUNTS: {
            // TODO
            break;
        }
        case GET_OFFER_ROW: {
            const char* symbol = (char*)message->payload;
            if(symbol == NULL) {
                char* errmsg = (char*)malloc(47*sizeof(char));
                strcpy(errmsg, "The symbol supplied to GET_OFFER_ROW was NULL!");
                *response = ServerMessage({ERROR, errmsg});
                break;
            }
            void* row = get_offer_row_log(session, symbol, env);
            if(row == NULL) {
                char* errmsg = (char*)malloc(44*sizeof(char));
                strcpy(errmsg, "The result from `get_offer_row()` was NULL!");
                *response = ServerMessage({ERROR, errmsg});
                break;
            }
            *response = ServerMessage({OFFER_ROW, row});
            break;
        }
        default: {
            char* errormsg = (char*)malloc(64*sizeof(char));
            strcpy(errormsg, "The broker server doesn't have a response for that command type");
            *response = ServerMessage({ERROR, (void*)errormsg});
            break;
        }
    }
}

/// Returns the current timestamp in microseconds.
long current_timestamp_micros() {
    ptime time_t_epoch(date(1970,1,1));
    ptime now = microsec_clock::universal_time();
    time_duration diff = now - time_t_epoch;
    return diff.total_microseconds();
}

/// Create the necessary broker connections and set up the environment for getting price updates
void init_tick_stream(void* cb_env, TickCallback cb, IO2GSession* session, Environment* env) {
    IO2GLoginRules* loginRules = session->getLoginRules();
    bool bLoaded = loginRules->isTableLoadedByDefault(Offers);
    env->tick_cb = cb;
    env->tick_cb_env = cb_env;

    if(bLoaded) {
        IO2GResponse* response = loginRules->getTableRefreshResponse(Offers);
    } else {
        IO2GRequestFactory* factory = session->getRequestFactory();
        IO2GRequest* refreshOffers = factory->createRefreshTableRequest(Offers);
        session->sendRequest(refreshOffers);
    }

    GlobalTableListener* gtl = new GlobalTableListener(cb, cb_env, env->log_cb, env->log_cb_env);
    IO2GTableManager* tableManager = session->getTableManager();
    O2GTableManagerStatus managerStatus = tableManager->getStatus();

    int retries = 0;
loadtables:
    char logmsg[] = "Loading tables...";
    while (managerStatus == TablesLoading) {
        rustlog(env, logmsg, NOTICE);
        Sleep(50);
        managerStatus = tableManager->getStatus();
    }
    char loadedmsg[] = "Tables are loaded.";
    rustlog(env, loadedmsg, NOTICE);

    if (managerStatus == TablesLoadFailed) {
        if(retries < 3) {
            char msg[] = "Cannot refresh all tables of table manager";
            rustlog(env, msg, ERR);
            retries += 1;
            goto loadtables;
        } else {
            char msg[] = "Can't load tables after 3 retries; unable to create broker server.";
            rustlog(env, msg, ERR);
            return;
        }
    } else if(managerStatus == TablesLoaded) {
        char msg[] = "Table manager has status TablesLoaded";
        rustlog(env, msg, DEBUG);
        std::cout << msg << std::endl;
    }

    if(tableManager == NULL) {
        char msg[] = "Table manager is NULL!!!";
        rustlog(env, msg, CRITICAL);
        std::cout << msg << std::endl;
        return;
    }

    gtl->subscribeNewOffers(tableManager);
}

/// Initializes the internal server event loop and starts listening for messages from the client.
void start_server(void* void_session, void* void_env) {
    Environment* env = (Environment*)void_env;
    IO2GSession* session = (IO2GSession*)void_session;
    ClientMessage message;
    ServerMessage response;

    while(true) {
        std::unique_lock<std::mutex> lock(*env->m);
        env->cond_var->wait(lock, [env, &message](){ return env->q->pop(message); });

        process_client_message(&message, &response, session, env);
        // send the response asynchronously back to the client if there is one to send.
        if(&response != NULL)
            env->cb(env->tx_ptr, &response);

        lock.unlock();
    }
}

/// Starts the internal server event loop and returns a reference to the queue that can be used along
/// with the `push_client_message()` function to send messages to it.
void* init_server_environment(ResponseCallback cb, void* tx_ptr, LogCallback log_cb, void* log_cb_env) {
    // heap allocate all the internals so they don't go out of scope and die
    MessageQueue* q = new MessageQueue();
    std::condition_variable* cond_var = new std::condition_variable();
    std::mutex* m = new std::mutex;
    GlobalResponseListener* g_response_listener = new GlobalResponseListener();
    Environment* heap_env = new Environment(
        {
            cb, // ResponseCallback
            tx_ptr, // tx_ptr
            q, // message queue
            cond_var, // condition variable
            m, // mutex
            NULL, // tick callback
            NULL, // tick callback environment
            NULL, // table manager
            log_cb, // log callback
            log_cb_env, // log callback environment
            NULL, // global table listener
            g_response_listener // global response listener
        }
    );

    return (void*)heap_env;
}

void rustlog(Environment* env, char* msg, CLogLevel severity) {
    if(env != NULL)
        env->log_cb(env->log_cb_env, msg, severity);
}
