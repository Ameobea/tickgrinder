//! A collection of helper functions that allow for communication with the FXCM ForexConnect API

#include <ctime>

#include "stdafx.h"
#include "libfxcm_ffi.h"
#include "ResponseListener.h"
#include "LoginParams.h"
#include "CommonSources.h"

/// Attempts to create a connection to the FXCM servers with the supplied credentials; returns a
/// nullptr if unsuccessful.
void* fxcm_login(char* username, char* password, char* url, bool live, LogCallback log_cb, void* log_cb_env){
    IO2GSession* session = CO2GTransport::createSession();
    session->useTableManager(Yes, NULL);
    SessionStatusListener *sessionListener = new SessionStatusListener(
        session, log_cb, log_cb_env
    );
    session->subscribeSessionStatus(sessionListener);
    sessionListener->reset();

    const char* conn_name = (live) ? "Live" : "Demo";
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
bool test_login(char* username, char* password, char* url, bool live){
    IO2GSession* session = (IO2GSession*)fxcm_login(username, password, url, live, NULL, NULL);
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
void print_accounts(IO2GSession* session){
    O2G2Ptr<IO2GResponseReaderFactory> readerFactory = session->getResponseReaderFactory();
    if (!readerFactory) {
        std::cout << "Cannot create response reader factory" << std::endl;
        return;
    }
    O2G2Ptr<IO2GLoginRules> loginRules = session->getLoginRules();
    O2G2Ptr<IO2GResponse> response = loginRules->getTableRefreshResponse(Accounts);
    O2G2Ptr<IO2GAccountsTableResponseReader> accountsResponseReader = readerFactory->createAccountsTableReader(response);
    std::cout.precision(2);
    for (int i = 0; i < accountsResponseReader->size(); ++i) {
        O2G2Ptr<IO2GAccountRow> accountRow = accountsResponseReader->getRow(i);
        std::cout << "AccountID: " << accountRow->getAccountID() << ", "
                << "Balance: " << std::fixed << accountRow->getBalance() << ", "
                << "Used margin: " << std::fixed << accountRow->getUsedMargin() << std::endl;
    }
};

void sendPrices(IO2GSession* session, IO2GResponse* response, void (*tickcallback)(void*, uint64_t, double, double), void* user_data) {
    O2G2Ptr<IO2GResponseReaderFactory> factory = session->getResponseReaderFactory();
    if (factory) {
        O2G2Ptr<IO2GMarketDataSnapshotResponseReader> reader = factory->createMarketDataSnapshotReader(response);
        if (reader) {
            uint64_t unix_time_ms;
            for (int i = reader->size() - 1; i >= 0; i--) {
                const double dt = reader->getDate(i);
                unix_time_ms = date_to_unix_ms(dt);

                // send the callback to RustLand
                tickcallback(user_data, unix_time_ms, reader->getBid(i), reader->getAsk(i));
            }
        } else {
            printf("No reader!\n");
        }
    } else {
        printf("No factory!\n");
    }
}

/// converts the given OLE Automation date (double) into milliseconds since the epoch (unix timestamp)
uint64_t date_to_unix_ms(DATE date) {
    struct tm tmBuf_inner;
    tm* tmBuf = &tmBuf_inner;
    SYSTEMTIME wt_inner;
    SYSTEMTIME* wt = &wt_inner;
    WORD ms;
    time_t tt;

    // convert crazy OLT time to SYSTEMTIME
    // OleTimeToWindowsTime(const double dt, SYSTEMTIME *st);
    bool success = CO2GDateUtils::OleTimeToWindowsTime(date, wt);
    if(!success){
        printf("Unable to convert OLE Time to Windows Time!\n");
    }
    if(wt == NULL){
        printf("date is null!");
    }
    // convert SYSTEMTIME to CTime which causes loss of ms precision
    // ms is simply dropped, not rounded.
    ms = wt->wMilliseconds;
    // WindowsTimeToCTime(const SYSTEMTIME *st, struct tm *t)
    CO2GDateUtils::WindowsTimeToCTime(wt, tmBuf);
    // convert to unix timestamp precise to the second
    tt = mktime(tmBuf);
    int unix_time_s = (int)tt;
    // convert to ms and add ms lost from before
    return (1000*(uint64_t)unix_time_s)+ms;
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

        IO2GRequest* request = reqFactory->createMarketDataSnapshotRequestInstrument(symbol, timeFrame, 300);
        ResponseListener* responseListener = new ResponseListener(session);
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
                        dateTo = reader->getDate(0); // earliest datetime of returned data
                    } else {
                        std::cout << "0 rows received" << std::endl;
                        break;
                    }
                }
                sendPrices(session, response, tickcallback, user_data);
            } else {
                printf("Received bad response type or no response at all.\n");
                break;
            }
        } while (dateTo - dateFrom > 0.0001);
        printf("After do/while\n");

        return true;
    } else {
        printf("Unable to connect to broker to download history.\n");
        return false;
    }
}

IO2GOfferRow* _getOffer(IO2GSession* session, const char* sInstrument, Environment* env) {
    if (!session || !sInstrument){
        rustlog(env, "Session or Instrument wasn't provided!", CRITICAL);
        return NULL;
    }

    O2G2Ptr<IO2GLoginRules> loginRules = session->getLoginRules();
    if (loginRules) {
        O2G2Ptr<IO2GResponse> response = loginRules->getTableRefreshResponse(Offers);
        if (response) {
            O2G2Ptr<IO2GResponseReaderFactory> readerFactory = session->getResponseReaderFactory();
            if (readerFactory) {
                O2G2Ptr<IO2GOffersTableResponseReader> reader = readerFactory->createOffersTableReader(response);

                for (int i = 0; i < reader->size(); ++i) {
                    O2G2Ptr<IO2GOfferRow> offer = reader->getRow(i);
                    if (offer) {
                        if (strcmp(sInstrument, offer->getInstrument()) == 0) {
                            if (strcmp(offer->getSubscriptionStatus(), "T") == 0) {
                                printf("Returning offer...\n");
                                return offer.Detach();
                            }
                        }
                        // std::cout << offer->getInstrument() << std::endl << sInstrument << std::endl;
                    } else {
                        rustlog(env, "offer was NULL!\n", WARNING);
                        printf("offer was NULL!\n");
                    }
                }
            } else {
                rustlog(env, "readerFactor was NULL!\n", CRITICAL);
                printf("readerFactor was NULL!\n");
            }
        } else {
            rustlog(env, "response was NULL!\n", CRITICAL);
            printf("response was NULL!\n");
        }
    } else {
        rustlog(env, "loginRules was NULL!\n", CRITICAL);
        printf("loginRules was NULL!\n");
    }

    rustlog(env, "Some other error occured while trying to get offer row!\n", CRITICAL);
    printf("Some other error occured while trying to get offer row!\n");
    return NULL;
}

/// Returns a void pointer to an OfferRow which can be used along with the other functions to
/// get information about current offers.
void* get_offer_row(void* void_session, const char* instrument){
    IO2GSession* session = (IO2GSession*)void_session;
    IO2GOfferRow* row = _getOffer(session, instrument, NULL);
    return row;
}

void* get_offer_row_log(IO2GSession* session, const char* instrument, Environment* env){
    IO2GOfferRow* row = _getOffer(session, instrument, env);

    return row;
}
