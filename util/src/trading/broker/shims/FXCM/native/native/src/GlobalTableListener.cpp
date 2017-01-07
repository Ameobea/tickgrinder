#include "stdafx.h"
#include "libfxcm_ffi.h"
#include <algorithm>
#include "GlobalTableListener.h"
#include <vector>

// Constructors and refcounting
GlobalTableListener::GlobalTableListener(TickCallback tcb, void* tce, LogCallback lcb, void* lcbe) {
    mRefCount = 1;
    tick_cb = tcb;
    tick_cb_env = tce;
    log_cb = lcb;
    log_cb_env = lcbe;
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
                CSymbolTick cst = {
                    offerRow->getInstrument(),
                    date_to_unix_ms(offerRow->getTime()),
                    offerRow->getBid(),
                    offerRow->getAsk(),
                };

                tick_cb(tick_cb_env, cst);
            } else {
                char msg[] = "Received invalid tick from the offers table";
                log_cb(log_cb_env, msg, WARNING);
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

void GlobalTableListener::subscribeTradingEvents(IO2GTableManager* manager) {
    O2G2Ptr<IO2GOrdersTable> ordersTable = (IO2GOrdersTable *)manager->getTable(Orders);

    ordersTable->subscribeUpdate(Insert, this);
}

void GlobalTableListener::unsubscribeTradingEvents(IO2GTableManager* manager) {
    O2G2Ptr<IO2GOrdersTable> ordersTable = (IO2GOrdersTable*)manager->getTable(Orders);

    ordersTable->unsubscribeUpdate(Insert, this);
}

void GlobalTableListener::subscribeNewOffers(IO2GTableManager* manager) {
    O2G2Ptr<IO2GOffersTable> offersTable = (IO2GOffersTable*)manager->getTable(Offers);

    offersTable->subscribeUpdate(Update, this);
}

void GlobalTableListener::unsubscribeNewOffers(IO2GTableManager* manager) {
    O2G2Ptr<IO2GOffersTable> offersTable = (IO2GOffersTable*)manager->getTable(Offers);

    offersTable->unsubscribeUpdate(Update, this);
}
