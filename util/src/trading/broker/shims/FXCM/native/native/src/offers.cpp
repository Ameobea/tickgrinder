//! Functions for dealing with offer rows, getting information from/about them and things like that.

#include "stdafx.h"
#include "CommonSources.h"

// For all the below functions, see the IO2GOfferRow documentation for their usage.
// http://www.fxcodebase.com/documents/ForexConnectAPI/IO2GOfferRow.html

double getBid(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return row->getBid();
}

const char *getBidTradable(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return row->getBidTradable();
}

double getAsk(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return row->getAsk();
}

const char *getAskTradable(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return row->getAskTradable();
}

extern "C" int getDigits(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return row->getDigits();
}

double getHigh(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return row->getHigh();
}

double getLow(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return row->getLow();
}

int getVolume(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return row->getVolume();
}

const char *getTradingStatus(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return row->getAskTradable();
}

double getPointSize(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return row->getPointSize();
}

double getPipSize(void* void_row) {
    IO2GOfferRow * row = (IO2GOfferRow*)void_row;
    return getPointSize(row);
}
