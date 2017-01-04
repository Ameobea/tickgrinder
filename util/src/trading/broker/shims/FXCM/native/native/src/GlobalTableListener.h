#pragma once

class GlobalTableListener : public IO2GTableListener {
public:
    GlobalTableListener(ResponseListener *responseListener);

    virtual long addRef();
    virtual long release();

    void setRequestIDs(std::vector<std::string> &orderIDs);

    void onStatusChanged(O2GTableStatus);
    void onAdded(const char*, IO2GRow*);
    void onChanged(const char*, IO2GRow*);
    void onDeleted(const char*, IO2GRow*);

    void subscribeTradingEvents(IO2GTableManager *manager);
    void unsubscribeTradingEvents(IO2GTableManager *manager);

private:
    long mRefCount;
    ResponseListener *mResponseListener;
    std::vector<std::string> mRequestIDs;

protected:
    virtual ~GlobalTableListener();
}
