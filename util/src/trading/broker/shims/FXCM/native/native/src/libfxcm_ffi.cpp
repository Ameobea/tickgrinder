//! A collection of helper functions that allow for communication with the FXCM ForexConnect API

#include <ctime>

#include "stdafx.h"
#include "ResponseListener.h"
#include "SessionStatusListener.h"
#include "LoginParams.h"
#include "CommonSources.h"

#include "libfxcm_ffi.h"

/// Attempts to create a connection to the FXCM servers with the supplied credentials; returns a
/// nullptr if unsuccessful.
void* fxcm_login(char *username, char *password, char *url, bool live){
    IO2GSession *session = CO2GTransport::createSession();
    SessionStatusListener *sessionListener = new SessionStatusListener(
        // session, false, session_id, pin
        session, false, 0, 0
    );
    session->subscribeSessionStatus(sessionListener);
    sessionListener->reset();

    const char *conn_name;
    if(live){
        conn_name = "Live";
    } else {
        conn_name = "Demo";
    }
    session->login(username, password, "http://www.fxcorporate.com/Hosts.jsp", conn_name);
    bool isConnected = sessionListener->waitEvents() && sessionListener->isConnected();
    if(isConnected){
        return (void*)session;
    } else {
        printf("Unable to connect to the broker.");
        return NULL;
    }
};

/// Connects to the broker and attempts to list the account balance.  Returns true if successful and false
/// if unsuccessful.
bool test_login(char *username, char *password, char *url, bool live){
    IO2GSession * session = (IO2GSession*)fxcm_login(username, password, url, live);
    if(session != NULL){
        print_accounts(session);
        session->logout();
        return true;
    } else {
        return false;
    }
}

/// Taken from FXCM examples in official repository
/// https://github.com/FXCMAPI/ForexConnectAPI-Linux-x86_64/blob/master/samples/cpp/NonTableManagerSamples/Login/source/main.cpp
void print_accounts(IO2GSession *session){
    O2G2Ptr<IO2GResponseReaderFactory> readerFactory = session->getResponseReaderFactory();
    if (!readerFactory)
    {
        std::cout << "Cannot create response reader factory" << std::endl;
        return;
    }
    O2G2Ptr<IO2GLoginRules> loginRules = session->getLoginRules();
    O2G2Ptr<IO2GResponse> response = loginRules->getTableRefreshResponse(Accounts);
    O2G2Ptr<IO2GAccountsTableResponseReader> accountsResponseReader = readerFactory->createAccountsTableReader(response);
    std::cout.precision(2);
    for (int i = 0; i < accountsResponseReader->size(); ++i)
    {
        O2G2Ptr<IO2GAccountRow> accountRow = accountsResponseReader->getRow(i);
        std::cout << "AccountID: " << accountRow->getAccountID() << ", "
                << "Balance: " << std::fixed << accountRow->getBalance() << ", "
                << "Used margin: " << std::fixed << accountRow->getUsedMargin() << std::endl;
    }
};

void sendPrices(IO2GSession *session, IO2GResponse *response, void (*tickcallback)(void*, uint64_t, double, double), void* user_data) {
    O2G2Ptr<IO2GResponseReaderFactory> factory = session->getResponseReaderFactory();
    if (factory) {
        O2G2Ptr<IO2GMarketDataSnapshotResponseReader> reader = factory->createMarketDataSnapshotReader(response);
        if (reader) {
            for (int i = reader->size() - 1; i >= 0; i--) {
                const double dt = reader->getDate(i);
                _SYSTEMTIME* wt = new _SYSTEMTIME();
                // convert crazy OLT time to SYSTEMTIME
                // OleTimeToWindowsTime(const double dt, SYSTEMTIME *st);
                bool success = CO2GDateUtils::OleTimeToWindowsTime(dt, wt);
                if(!success){
                    printf("Unable to convert OLE Time to Windows Time!\n");
                }
                if(wt == NULL){
                    printf("dt is null!");
                }
                tm *tmBuf = new tm();
                // convert SYSTEMTIME to CTime which causes loss of ms precision
                // ms is simply dropped, not rounded.
                WORD ms = wt->wMilliseconds;
                // WindowsTimeToCTime(const SYSTEMTIME *st, struct tm *t)
                CO2GDateUtils::WindowsTimeToCTime(wt, tmBuf);
                // convert to unix timestamp precise to the second
                time_t tt = mktime(tmBuf);
                int unix_time_s = (int)tt;
                // convert to ms and add ms lost from before
                uint64_t unix_time_ms = (1000*(uint64_t)unix_time_s)+ms;
                double bid = reader->getBid(i);
                double ask = reader->getAsk(i);

                // send the callback to RustLand
                tickcallback(user_data, unix_time_ms, bid, ask);
            }
        } else {
            printf("No reader!\n");
        }
    } else {
        printf("No factory!\n");
    }
}

/// Initializes a history downloader instance.  It takes a function is called as a callback for every tick downloaded.
bool init_history_download(
    void* void_session,
    char* symbol,
    char* startTime,
    char* endTime,
    void (*tickcallback)(void*, uint64_t, double, double),
    void* user_data
){
    IO2GSession* session = (IO2GSession*)void_session;
    if(session != NULL){
        O2G2Ptr<IO2GRequestFactory> reqFactory = session->getRequestFactory();
        IO2GTimeframeCollection * timeFrames = reqFactory->getTimeFrameCollection();
        IO2GTimeframe * timeFrame = timeFrames->get("t1");

        DATE dateFrom, dateTo;
        struct tm tmBuf = {0};
        // convert the input time char arrays to `DATE`s
        strptime(startTime, "%m.%d.%Y %H:%M:%S", &tmBuf);
        CO2GDateUtils::CTimeToOleTime(&tmBuf, &dateFrom);
        strptime(endTime, "%m.%d.%Y %H:%M:%S", &tmBuf);
        CO2GDateUtils::CTimeToOleTime(&tmBuf, &dateTo);

        IO2GRequest * request = reqFactory->createMarketDataSnapshotRequestInstrument("EUR/USD", timeFrame, 300);
        ResponseListener *responseListener = new ResponseListener(session);
        session->subscribeResponse(responseListener);
        do {
            reqFactory->fillMarketDataSnapshotRequestTime(request, dateFrom, dateTo, false);
            responseListener->setRequestID(request->getRequestID());
            session->sendRequest(request);
            if (!responseListener->waitEvents()) {
                std::cout << "Response waiting timeout expired" << std::endl;
                continue;
            }
            // shift "to" bound to oldest datetime of returned data
            O2G2Ptr<IO2GResponse> response = responseListener->getResponse();
            if (response && response->getType() == MarketDataSnapshot) {
                O2G2Ptr<IO2GResponseReaderFactory> readerFactory = session->getResponseReaderFactory();
                if (readerFactory) {
                    O2G2Ptr<IO2GMarketDataSnapshotResponseReader> reader = readerFactory->createMarketDataSnapshotReader(response);
                    if (reader->size() > 0) {
                        // if (abs(dateTo - reader->getDate(0)) > 0) {
                            dateTo = reader->getDate(0); // earliest datetime of returned data
                        // } else {
                        //     // printf("breaking...\n");
                        //     // break;
                        // }
                    } else {
                        std::cout << "0 rows received" << std::endl;
                        break;
                    }
                }
                sendPrices(session, response, tickcallback, user_data);
            } else {
                printf("Received bad resposne type or no response at all.\n");
                std::cout << "Response type: " << response->getType() << std::endl;
                break;
            }
        } while (dateTo - dateFrom > 0.0001);
        printf("After do/while\n");

        session->logout();
        return true;
    } else {
        printf("Unable to connect to broker to download history.\n");
        return false;
    }
}

/// Returns a void pointer to an OfferRow which can be used along with the other functions to
/// get information about current offers.
void* get_offer_row(void* void_session, const char *instrument){
    IO2GSession * session = (IO2GSession*)void_session;
    IO2GOfferRow * row = getOffer(session, instrument);
    return row;
}
