#include "stdafx.h"
#include "libfxcm_ffi.h"

SessionStatusListener::SessionStatusListener(IO2GSession *session, LogCallback _log_cb, void* _log_cb_env) {
    mSession = session;
    mSession->addRef();
    reset();
    mRefCount = 1;
    mSessionEvent = CreateEvent(0, FALSE, FALSE, 0);
    mSessionID = "";
    log_cb = _log_cb;
    // if this is NULL, logging will be disabled.
    log_cb_env = _log_cb_env;
}

SessionStatusListener::~SessionStatusListener() {
    mSessionID.clear();
    mSession->release();
    CloseHandle(mSessionEvent);
}

long SessionStatusListener::addRef() {
    return InterlockedIncrement(&mRefCount);
}

long SessionStatusListener::release() {
    long rc = InterlockedDecrement(&mRefCount);
    if (rc == 0)
        delete this;
    return rc;
}

void SessionStatusListener::reset() {
    mConnected = false;
    mDisconnected = false;
    mError = false;
}

void SessionStatusListener::onLoginFailed(const char *error) {
    char msg[] = "Login error: ";
    char* msg_concat = (char*)malloc(strlen(msg)+1+strlen(error));
    strcpy(msg_concat, msg);
    strcat(msg_concat, error);
    _log(msg_concat, ERR);
    mError = true;
}

void SessionStatusListener::onSessionStatusChanged(IO2GSessionStatus::O2GSessionStatus status) {
    switch (status) {
    case IO2GSessionStatus::Disconnected:
        _log("Session status: status::disconnected", WARNING);
        mConnected = false;
        mDisconnected = true;
        SetEvent(mSessionEvent);
        break;
    case IO2GSessionStatus::Connecting:
        _log("Session status: status::connecting", DEBUG);
        break;
    case IO2GSessionStatus::TradingSessionRequested: {
        _log("Session status: status::trading session requested", DEBUG);
        O2G2Ptr<IO2GSessionDescriptorCollection> descriptors = mSession->getTradingSessionDescriptors();
        bool found = false;
        if (descriptors) {
            for (int i = 0; i < descriptors->size(); ++i) {
                O2G2Ptr<IO2GSessionDescriptor> descriptor = descriptors->get(i);
                if (mSessionID == descriptor->getID()) {
                    found = true;
                    break;
                }
            }
        }

        if (!found) {
            onLoginFailed("The specified sub session identifier is not found");
        } else {
            mSession->setTradingSession(mSessionID.c_str(), "");
        }
    }
    break;
    case IO2GSessionStatus::Connected:
        _log("Session status: status::connected", DEBUG);
        mConnected = true;
        mDisconnected = false;
        SetEvent(mSessionEvent);
        break;
    case IO2GSessionStatus::Reconnecting:
        _log("Session status: status::reconnection", DEBUG);
        break;
    case IO2GSessionStatus::Disconnecting:
        _log("Session status: status::disconnecting", DEBUG);
        break;
    case IO2GSessionStatus::SessionLost:
        _log("Session status: Session Lost!", ERR);
        break;
    }
}

bool SessionStatusListener::hasError() const {
    return mError;
}

bool SessionStatusListener::isConnected() const {
    return mConnected;
}

bool SessionStatusListener::isDisconnected() const {
    return mDisconnected;
}

bool SessionStatusListener::waitEvents() {
    return WaitForSingleObject(mSessionEvent, _TIMEOUT) == 0;
}

void SessionStatusListener::_log(char* msg, CLogLevel severity) {
    if(log_cb_env != NULL)
        log_cb(log_cb_env, msg, severity);
}
