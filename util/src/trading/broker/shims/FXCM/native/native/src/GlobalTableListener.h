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

    void subscribeTradingEvents(IO2GTableManager *manager);
    void unsubscribeTradingEvents(IO2GTableManager *manager);
    void subscribeNewOffers(IO2GTableManager *manager);
    void unsubscribeNewOffers(IO2GTableManager *manager);

private:
    long mRefCount;
    TickCallback tick_cb;
    void* tick_cb_env;
    LogCallback log_cb;
    void* log_cb_env;
};
