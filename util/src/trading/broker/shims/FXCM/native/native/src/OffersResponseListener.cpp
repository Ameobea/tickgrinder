#include "stdafx.h"
#include <math.h>
#include <algorithm>

#include <sstream>
#include <iomanip>
#include "OffersResponseListener.h"

OffersResponseListener::OffersResponseListener() {
    mRefCount = 1;
    mResponseEvent = CreateEvent(0, FALSE, FALSE, 0);
}

OffersResponseListener::~OffersResponseListener() {
    CloseHandle(mResponseEvent);
}

long OffersResponseListener::addRef() {
    return InterlockedIncrement(&mRefCount);
}

long OffersResponseListener::release() {
    long rc = InterlockedDecrement(&mRefCount);
    if (rc == 0)
        delete this;
    return rc;
}

void OffersResponseListener::setRequestIDs(std::vector<std::string> &requestIDs) {
    mRequestIDs.resize(requestIDs.size());
    std::copy(requestIDs.begin(), requestIDs.end(), mRequestIDs.begin());
    ResetEvent(mResponseEvent);
}

bool OffersResponseListener::waitEvents() {
    return WaitForSingleObject(mResponseEvent, _TIMEOUT) == 0;
}

void OffersResponseListener::stopWaiting() {
    SetEvent(mResponseEvent);
}

void OffersResponseListener::onRequestCompleted(const char *requestId, IO2GResponse *response) {
    // TODO???
}

void OffersResponseListener::onRequestFailed(const char *requestId , const char *error) {
    if (std::find(mRequestIDs.begin(), mRequestIDs.end(), requestId) != mRequestIDs.end()) {
        // std::cout << "The request has been failed. ID: " << requestId << " : " << error << std::endl;
        // TODO: Log to Rust
        stopWaiting();
    }
}

/** Request update data received data handler. */
void ResponseListener::onTablesUpdates(IO2GResponse *data) {
    // TODO??¿
}
