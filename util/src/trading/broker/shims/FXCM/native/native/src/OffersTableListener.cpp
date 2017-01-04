#include "stdafx.h"
#include <algorithm>
#include "OffersResponseListener.h"
#include "OffersTableListener.h"

OffersTableListener::OffersTableListener(ResponseListener *responseListener) {
    mRefCount = 1;
    responseListener->addRef();
}

OffersTableListener::~OffersTableListener(void) {
    if (mResponseListener)
        mResponseListener->release();
}

long OffersTableListener::addRef() {
    return InterlockedIncrement(&mRefCount);
}

long OffersTableListener::release() {
    InterlockedDecrement(&mRefCount);
    if(mRefCount == 0)
        delete this;
    return mRefCount;
}

/** Set request. */
void OffersTableListener::setRequestIDs(std::vector<std::string> &requestIDs) {
    mRequestIDs.resize(requestIDs.size());
    std::copy(requestIDs.begin(), requestIDs.end(), mRequestIDs.begin());
}

void OffersTableListener::onAdded(const char *rowID, IO2GRow *row) {
    if (row->getTableType() == Orders) {
        IO2GOrderRow *order = static_cast<IO2GOrderRow *>(row);
        std::vector<std::string>::iterator iter;
        iter = std::find(mRequestIDs.begin(), mRequestIDs.end(), order->getRequestID());
        if (iter != mRequestIDs.end()) {
            // TODO (eventually)
            mRequestIDs.erase(iter);
            if (mRequestIDs.size() == 0)
                mResponseListener->stopWaiting();
        }
    }
}

void OffersTableListener::onChanged(const char *rowID, IO2GRow *row) {
    // TODO
}

void OffersTableListener::onDeleted(const char *rowID, IO2GRow *row) {

}

void OffersTableListener::onStatusChanged(O2GTableStatus status) {
    // TODO
}

void OffersTableListener::subscribeEvents(IO2GTableManager *manager) {
    O2G2Ptr<IO2GOrdersTable> ordersTable = (IO2GOrdersTable *)manager->getTable(Orders);

    ordersTable->subscribeUpdate(Insert, this);
}

void OffersTableListener::unsubscribeEvents(IO2GTableManager *manager) {
    O2G2Ptr<IO2GOrdersTable> ordersTable = (IO2GOrdersTable *)manager->getTable(Orders);

    ordersTable->unsubscribeUpdate(Insert, this);
}
