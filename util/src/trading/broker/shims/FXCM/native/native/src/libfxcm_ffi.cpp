//! A collection of helper functions that allow for communication with the FXCM ForexConnect API

#include "stdafx.h"
#include "SessionStatusListener.h"
#include "LoginParams.h"
#include "CommonSources.h"

#include "libfxcm_ffi.h"

void fxcm_login(char *username, char *password, char *url, bool live){
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
        print_accounts(session);
        std::cout << "Done!" << std::endl;
        sessionListener->reset();
        session->logout();
        sessionListener->waitEvents();
    } else {
        printf("Unable to connect to the broker.");
    }
};

// Taken from FXCM examples in official repository
// https://github.com/FXCMAPI/ForexConnectAPI-Linux-x86_64/blob/master/samples/cpp/NonTableManagerSamples/Login/source/main.cpp
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
