#pragma once

class GlobalTableListener : public IO2GTableListener {
public:
    GlobalTableListener(TickCallback tcb, void* tce, LogCallback lcb, void* lcbe);

    virtual long addRef();
    virtual long release();

    void setRequestIDs(std::vector<std::string> &orderIDs);

    void onStatusChanged(O2GTableStatus);
    void onAdded(const char*, IO2GRow*);
    void onChanged(const char*, IO2GRow*);
    void onDeleted(const char*, IO2GRow*);

    void setTickCallback(TickCallback _tcb, void* _tcbe);
    void setResponseCallback(ResponseCallback rcb, void* rcbe);

    void subscribeTradingEvents(IO2GTableManager *manager);
    void unsubscribeTradingEvents(IO2GTableManager *manager);
    void subscribeNewOffers(IO2GTableManager *manager);
    void unsubscribeNewOffers(IO2GTableManager *manager);

private:
    long mRefCount;

    // tick callback
    TickCallback tick_cb;
    void* tick_cb_env;

    // log callback
    LogCallback log_cb;
    void* log_cb_env;

    // response callback
    ResponseCallback res_cb;
    void* res_cb_env;
};
