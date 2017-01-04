#pragma once

class OffersTableListener : public IO2GTableListener {
public:
    OffersTableListener(ResponseListener *responseListener);

    virtual long addRef();
    virtual long release();

    void setRequestIDs(std::vector<std::string> &orderIDs);

    void onStatusChanged(O2GTableStatus);
    void onAdded(const char*, IO2GRow*);
    void onChanged(const char*, IO2GRow*);
    void onDeleted(const char*, IO2GRow*);

    void subscribeEvents(IO2GTableManager *manager);
    void unsubscribeEvents(IO2GTableManager *manager);

private:
    long mRefCount;
    ResponseListener *mResponseListener;
    std::vector<std::string> mRequestIDs;

 protected:
    /** Destructor. */
    virtual ~OffersTableListener();
}
