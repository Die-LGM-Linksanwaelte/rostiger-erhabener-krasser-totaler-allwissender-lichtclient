///enum to describe the connection state of the user
#[derive(PartialEq, Debug, Clone)]
pub enum ConnectionState {
    Connected {
        session_state: SessionState,
    },
    Disconnected,
    Error,
    ConnectionPending,
}

///enum to describe the session state of the user
#[derive(PartialEq, Debug, Clone)]
pub enum SessionState {
    LoginPending,
    LoggedOut,
    LoggedIn,
    LoginFailed(String),
}
