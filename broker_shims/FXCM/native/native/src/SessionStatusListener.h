#pragma once

class SessionStatusListener : public IO2GSessionStatus {
private:
    long mRefCount;
    bool mError;
    bool mConnected;
    bool mDisconnected;
    IO2GSession *mSession;
    /** Event handle. */
    HANDLE mSessionEvent;
    LogCallback log_cb;
    void* log_cb_env;
    std::string mSessionID;

protected:
    ~SessionStatusListener();

public:
    SessionStatusListener(IO2GSession *session, LogCallback log_cb, void* log_cb_env);

    virtual long addRef();
    virtual long release();

    virtual void onLoginFailed(const char *error);
    virtual void onSessionStatusChanged(IO2GSessionStatus::O2GSessionStatus status);

    bool hasError() const;
    bool isConnected() const;
    bool isDisconnected() const;
    void reset();
    bool waitEvents();

    void _log(char* msg, CLogLevel severity);
};
