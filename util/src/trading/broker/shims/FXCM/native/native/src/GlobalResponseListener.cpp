#include "stdafx.h"
#include <math.h>
#include <algorithm>

#include <sstream>
#include <iomanip>
#include "GlobalResponseListener.h"

GlobalResponseListener::GlobalResponseListener() {
    mRefCount = 1;
    mResponseEvent = CreateEvent(0, FALSE, FALSE, 0);
}

GlobalResponseListener::~GlobalResponseListener() {
    CloseHandle(mResponseEvent);
}

long GlobalResponseListener::addRef() {
    return InterlockedIncrement(&mRefCount);
}

long GlobalResponseListener::release() {
    long rc = InterlockedDecrement(&mRefCount);
    if (rc == 0)
        delete this;
    return rc;
}

void GlobalResponseListener::setRequestIDs(std::vector<std::string> &requestIDs) {
    mRequestIDs.resize(requestIDs.size());
    std::copy(requestIDs.begin(), requestIDs.end(), mRequestIDs.begin());
    ResetEvent(mResponseEvent);
}

bool GlobalResponseListener::waitEvents() {
    return WaitForSingleObject(mResponseEvent, _TIMEOUT) == 0;
}

void GlobalResponseListener::stopWaiting() {
    SetEvent(mResponseEvent);
}

void GlobalResponseListener::onRequestCompleted(const char* requestId, IO2GResponse *response) {
    // TODO???
}

void GlobalResponseListener::onRequestFailed(const char* requestId , const char* error) {
    if (std::find(mRequestIDs.begin(), mRequestIDs.end(), requestId) != mRequestIDs.end()) {
        // std::cout << "The request has been failed. ID: " << requestId << " : " << error << std::endl;
        // TODO: Log to Rust
        stopWaiting();
    }
}

void ResponseListener::onTablesUpdates(IO2GResponse* data) {}
