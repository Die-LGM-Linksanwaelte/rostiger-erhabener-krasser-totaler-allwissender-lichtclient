#[derive(PartialEq, Debug, Clone)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Error,
    ConnectionPending,
}

#[derive(PartialEq, Debug, Clone)]
pub enum SessionState {
    LoginPending,
    LoggedOut,
    LoggedIn,
    LoginFailed(String),
}
