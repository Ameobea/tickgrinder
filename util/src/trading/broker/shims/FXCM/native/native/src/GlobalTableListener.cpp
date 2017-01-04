#include "stdafx.h"
#include <algorithm>
#include "OffersResponseListener.h"
#include "GlobalTableListener.h"
#include <vector>

// default constructors and refcounting
GlobalTableListener::GlobalTableListener(ResponseListener *responseListener) {
    mRefCount = 1;
    responseListener->addRef();
}

GlobalTableListener::~GlobalTableListener(void) {
    if (mResponseListener)
        mResponseListener->release();
}

long GlobalTableListener::addRef() {
    return InterlockedIncrement(&mRefCount);
}

long GlobalTableListener::release() {
    InterlockedDecrement(&mRefCount);
    if(mRefCount == 0)
        delete this;
    return mRefCount;
}

void GlobalTableListener::onAdded(const char *rowID, IO2GRow *row) {}

/// This is where the action happens.  This is called every time a row is changed in one of the
/// tables that this listener is watching.  We determine the type of the update with `getTableType()`
/// and process it accordingly.
void GlobalTableListener::onChanged(const char *rowID, IO2GRow *row) {
    switch(row->getTableType()) {
        Offers: {
            IO2GOfferRow* offerRow = (IO2GOfferRow*)row;
            if(offerRow->isBidValid() && offerRow->isAskValid() && offerRow->isTimeValid() && 
                    offerRow->isInstrumentValid()){
                CSymbolTick cst = CSymbolTick({
                    offerRow->getInstrument,
                    date_to_unix_ms(offerRow->getTime()),
                    offerRow->getBid(),
                    offerRow->getAsk(),
                });
                // TODO: Send tick via callback
            } else {
                // TODO: Log error
            }
            break;
        }
        Orders: {
            IO2GOrderTableRow* orderRow = (IO2GOrderTableRow*)row;
            // TODO
            break;
        }
        Messages: {
            IO2GMessageTableRow* messageRow = (IO2GMessageTableRow*)row;
            // TODO
            break;
        }
    }
}

void GlobalTableListener::onDeleted(const char *rowID, IO2GRow *row) {}

void GlobalTableListener::onStatusChanged(O2GTableStatus status) {
    // TODO
}

void GlobalTableListener::subscribeTradingEvents(IO2GTableManager *manager) {
    O2G2Ptr<IO2GOrdersTable> ordersTable = (IO2GOrdersTable *)manager->getTable(Orders);

    ordersTable->subscribeUpdate(Insert, this);
}

void GlobalTableListener::unsubscribeTradingEvents(IO2GTableManager *manager) {
    O2G2Ptr<IO2GOrdersTable> ordersTable = (IO2GOrdersTable *)manager->getTable(Orders);

    ordersTable->unsubscribeUpdate(Insert, this);
}
