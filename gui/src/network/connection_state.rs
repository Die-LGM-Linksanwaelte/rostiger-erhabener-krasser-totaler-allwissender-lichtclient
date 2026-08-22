//! # Connection State Module
//!
//! This module defines the core data structures used to track the GUI application's
//! network connection status and user authentication session state.

/// Describes the physical network connection state to the kernel server.
#[derive(PartialEq, Debug, Clone)]
pub enum ConnectionState {
    /// Active network connection to the server, carrying a nested [`SessionState`].
    Connected {
        /// The active user authentication session status.
        session_state: SessionState,
    },
    /// No network connection is active.
    Disconnected,
    /// Connection attempt failed or an error occurred on the TCP socket.
    Error,
    /// Connection attempt is currently in progress (e.g. TCP handshake ongoing).
    ConnectionPending,
}

/// Describes the user authentication session state within an active connection.
#[derive(PartialEq, Debug, Clone)]
pub enum SessionState {
    /// Authentication request has been sent to the server, awaiting response.
    LoginPending,
    /// Connected to the server, but no user is currently authenticated.
    LoggedOut,
    /// Successfully authenticated and logged in to the server.
    LoggedIn,
    /// Authentication attempt failed, carrying the failure reason string provided by the server.
    LoginFailed(String),
}
