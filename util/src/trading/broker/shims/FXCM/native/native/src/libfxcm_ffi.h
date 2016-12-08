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
