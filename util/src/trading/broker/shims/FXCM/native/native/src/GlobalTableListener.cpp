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

void GlobalTableListener::onAdded(const char* rowID, IO2GRow* row) {
    char msg[] = "New row was added to the Offers table!";
    log_cb(log_cb_env, msg, WARNING);
}

/// This is where the action happens.  This is called every time a row is changed in one of the
/// tables that this listener is watching.  We determine the type of the update with `getTableType()`
/// and process it accordingly.
void GlobalTableListener::onChanged(const char* rowID, IO2GRow* row) {
    // char msg[] = "New update received by the GTL of type: ";
    // std::cout << msg << row->getTableType() << std::endl;
    switch(row->getTableType()) {
        case Offers: {
            IO2GOfferRow* offerRow = (IO2GOfferRow*)row;
            if(
                offerRow->isBidValid() && offerRow->isAskValid() &&
                offerRow->isTimeValid() && offerRow->isInstrumentValid()
            ){
                CSymbolTick cst;
                cst.symbol = offerRow->getInstrument();
                cst.timestamp = current_timestamp_micros();
                cst.bid = offerRow->getBid();
                cst.ask = offerRow->getAsk();

                // std::cout << "Received tick with price: " << offerRow->getBid() << std::endl;
                tick_cb(tick_cb_env, cst);
            } else {
                char msg[] = "Received invalid tick from the offers table";
                log_cb(log_cb_env, msg, WARNING);
            }
            break;
        }
        case Orders: {
            IO2GOrderTableRow* orderRow = (IO2GOrderTableRow*)row;
            // TODO
            break;
        }
        case Messages: {
            IO2GMessageTableRow* messageRow = (IO2GMessageTableRow*)row;
            // TODO
            break;
        }
    }
}

void GlobalTableListener::onDeleted(const char* rowID, IO2GRow* row) {
    char msg[] = "Row was deleted from the Offers table!";
    log_cb(log_cb_env, msg, WARNING);
}

void GlobalTableListener::onStatusChanged(O2GTableStatus status) {
    char* msg = (char*)malloc(sizeof(char)*52);
    switch(status) {
        case Failed: {
            strcpy(msg, "Global Table Listener status changed to: Failed");
            break;
        }
        case Initial: {
            strcpy(msg, "Global Table Listener status changed to: Initial");
            break;
        }
        case Refreshed: {
            strcpy(msg, "Global Table Listener status changed to: Refreshed");
            break;
        }
        case Refreshing: {
            strcpy(msg, "Global Table Listener status changed to: Refreshing");
            break;
        }
    }
    log_cb(log_cb_env, msg, DEBUG);
}

void GlobalTableListener::subscribeTradingEvents(IO2GTableManager* manager) {
    O2G2Ptr<IO2GOrdersTable> ordersTable = (IO2GOrdersTable*)manager->getTable(Orders);

    ordersTable->subscribeUpdate(Insert, this);
}

void GlobalTableListener::unsubscribeTradingEvents(IO2GTableManager* manager) {
    O2G2Ptr<IO2GOrdersTable> ordersTable = (IO2GOrdersTable*)manager->getTable(Orders);

    ordersTable->unsubscribeUpdate(Insert, this);
}

void GlobalTableListener::subscribeNewOffers(IO2GTableManager* manager) {
    O2G2Ptr<IO2GOffersTable> offersTable = (IO2GOffersTable*)manager->getTable(Offers);

    offersTable->subscribeUpdate(Update, this);
    offersTable->subscribeStatus(this);
    char msg[] = "Global Table Listener has subscribed to new offers.";
    log_cb(log_cb_env, msg, DEBUG);
}

void GlobalTableListener::unsubscribeNewOffers(IO2GTableManager* manager) {
    O2G2Ptr<IO2GOffersTable> offersTable = (IO2GOffersTable*)manager->getTable(Offers);

    offersTable->unsubscribeUpdate(Update, this);
    offersTable->unsubscribeStatus(this);
}
