#include <mutex>
#include <condition_variable>

#include "stdafx.h"
#include <boost/lockfree/spsc_queue.hpp>

enum CLogLevel {
    DEBUG,
    NOTICE,
    WARNING,
    ERR,
    CRITICAL,
};

typedef void (*LogCallback)(void* env_ptr, char* message, CLogLevel severity);

// really returns a IO2GSession*
extern "C" void* fxcm_login(char *username, char *password, char *url, bool live, LogCallback log_cb, void* log_cb_env);
void print_accounts(IO2GSession *session);
extern "C" bool test_login(char *username, char *password, char *url, bool live);

extern "C" bool init_history_download(
    void* void_session,
    char* symbol,
    char* startTime,
    char* endTime,
    void (*tickcallback)(void*, uint64_t, double, double),
    void* user_data
);

// Offer Functions
extern "C" void* get_offer_row(void* void_session, const char *instrument);

extern "C" double getBid(void* row);
extern "C" const char *getBidTradable(void* row);
extern "C" double getAsk(void* row);
extern "C" const char *getAskTradable(void* row);
extern "C" int getDigits(void* row);
extern "C" double getHigh(void* row);
extern "C" double getLow(void* row);
extern "C" int getVolume(void* row);
extern "C" const char *getTradingStatus(void* row);
extern "C" double getPointSize(void* row);
extern "C" double getPipSize(void* row);

// broker server

/// Contains all possible commands that can be received by the broker server.
enum ServerCommand {
    MARKET_OPEN,
    MARKET_CLOSE,
    LIST_ACCOUNTS,
    DISCONNECT,
    PING,
    INIT_TICK_SUB,
    GET_OFFER_ROW,
};

/// Contains all possible responses that can be sent by the broker server.
enum ServerResponse {
    POSITION_OPENED,
    POSITION_CLOSED,
    ORDER_PLACED,
    ORDER_REMOVED,
    SESSION_TERMINATED,
    PONG,
    ERROR,
    TICK_SUB_SUCCESSFUL,
    OFFER_ROW,
};

struct ServerMessage {
    ServerResponse response;
    void* payload;
};

struct ClientMessage {
    ServerCommand command;
    void* payload;
};

struct CSymbolTick {
    const char* symbol;
    uint64_t timestamp;
    double bid;
    double ask;
};

typedef void (*ResponseCallback)(void* tx_ptr, ServerMessage* res);
typedef void (*TickCallback)(void* env_ptr, CSymbolTick cst);
typedef boost::lockfree::spsc_queue<ClientMessage, boost::lockfree::capacity<1024> > MessageQueue;

#include "GlobalTableListener.h"
#include "GlobalResponseListener.h"
#include "SessionStatusListener.h"

/// Contains pointers to a bunch of heap-allocated interal variables that are used by the server
/// to maintain state, provide synchronization, and store messages.  The whole thing should be
/// Threadsafe (+Sync) so we can pass it around everywhere with impunity.
struct Environment {
    ResponseCallback cb;
    void* tx_ptr; // TODO: Make sure this is +Sync in Rust; we may need to Mutex-wrap it Rust-side
    MessageQueue* q;
    std::condition_variable* cond_var;
    std::mutex* m;
    TickCallback tick_cb;
    void* tick_cb_env;
    IO2GTableManager* tableManager;
    LogCallback log_cb;
    void* log_cb_env;
    GlobalTableListener* g_table_listener;
    GlobalResponseListener* g_response_listener;
};

/// Contains data necessary to initialize a tickstream
struct TickstreamDef {
    void* env_ptr;
    TickCallback cb;
};

/// returns a server environment that is sent along with BrokerCommands to access the server
extern "C" void* init_server_environment(void (*cb)(void* tx_ptr, ServerMessage* res), void* tx_ptr, LogCallback log_cb, void* log_cb_env);
/// starts the server event loop, blocking and waiting for messages from the client.
extern "C" void start_server(void* void_session, void* void_env);
/// sends a message to the server to be processed
extern "C" void push_client_message(ClientMessage msg, void* void_env);

// internal use only
void init_tick_stream(void* tx_ptr, TickCallback cb, IO2GSession* session, Environment* env);
uint64_t date_to_unix_ms(DATE date);
void rustlog(Environment* env, char* msg, CLogLevel severity);
void* get_offer_row_log(IO2GSession* session, const char *instrument, Environment* env);
