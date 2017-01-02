#include "stdafx.h"

// really returns a IO2GSession*
extern "C" void* fxcm_login(char *username, char *password, char *url, bool live);
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
};

struct ServerMessage {
    ServerResponse response;
    void* payload;
};

struct ClientMessage {
    ServerCommand command;
    void* payload;
};

/// returns a server environment that is sent along with BrokerCommands to access the server
extern "C" void* init_server_environment(void (*cb)(void* tx_ptr, ServerMessage* res), void* tx_ptr);
/// starts the server event loop, blocking and waiting for messages from the client.
extern "C" void start_server(void* void_session, void* void_env);
/// takes a reference to the server environment, a command, and the command's arguments and executes it
extern "C" void exec_command(ServerCommand command, void* args, void* env);
/// sends a message to the server to be processed
extern "C" void push_client_message(ClientMessage msg, void* void_env);
