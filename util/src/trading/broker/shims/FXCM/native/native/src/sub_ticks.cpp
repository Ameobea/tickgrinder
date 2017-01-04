//! Logic for getting streaming price updates.

/// Create the necessary broker connections and set up the environment for getting price updates
void init_tick_stream(const char* init_symbol, void* tx_ptr, TickCallback cb, IO2GSession* session) {
    ResponseListener* responseListener = new OffersResponseListener(/* TODO */);
    IO2GLoginRules* loginRules = session->getLoginRules();
    bool bLoaded = loginRules->isTableLoadedByDefault(Offers);
    if(bLoaded) {
        IO2GResponse* response = loginRules->getTableRefeshResponse(Offers);
    } else {
        IO2GRequestFactory* factory = session->getRequestFactory();
        IO2GRequest * refreshOffers = factory->createRefreshTableRequest(Offers);
        session->sendRequest(refreshOffers);
        // Finally capture IO2GResponse in the onRequestCompleted function of a response listener class.
        // You should also capture IO2GResponse in the onTablesUpdates function of a response listener class.
    }
}

/// Adds a symbol to the list of symbols that will receive updates
void add_symbol(const char* symbol) {

}
