#pragma once

class OffersResponseListener : public IO2GTableListener {
public:
    OffersResponseListener();
    virtual long addRef();
    virtual long release();

    void setRequestIDs(std::vector<std::string> &orderIDs);

    bool waitEvents();
    void stopWaiting();

    virtual void onRequestCompleted(const char *requestId, IO2GResponse *response = 0);

    virtual void onRequestFailed(const char *requestId , const char *error);

    virtual void onTablesUpdates(IO2GResponse *data);

private:
    long mRefCount;
    std::vector<std::string> mRequestIDs;
    HANDLE mResponseEvent;

protected:
    /** Destructor. */
    virtual ~TableListener();
};
