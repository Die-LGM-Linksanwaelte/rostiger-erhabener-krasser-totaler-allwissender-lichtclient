#[derive(PartialEq, Debug, Clone)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Error,
    ConnectionPending,
    LoginPending,
    LoggedOut,
    LoggedIn,
    LoginFailed(String),
}
