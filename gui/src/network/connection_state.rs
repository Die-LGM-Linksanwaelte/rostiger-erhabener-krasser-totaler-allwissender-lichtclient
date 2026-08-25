//! # Connection State Module
//!
//! This module defines the core data structures used to track the GUI application's
//! network connection status and user authentication session state.

use std::fmt;
use std::fmt::{Debug, Display, Formatter};

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

/// Formats [`ConnectionState`] for human-readable display.
///
/// The output delegates to [`SessionState`]'s [`Display`] implementation when the
/// connection is active, so the full state reads e.g. `"Connected and Log in"`.
impl Display for ConnectionState {
    /// Writes a human-readable representation of the connection state to `f`.
    ///
    /// # Variants
    /// - `Connected { session_state }` → `"Connected and <session_state>"`
    /// - `Disconnected`                → `"Disconnected"`
    /// - `Error`                       → `"Error"`
    /// - `ConnectionPending`           → `"Connection Pending"`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionState::Connected { session_state } => write!(f, "Connected and {}", session_state),
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Error => write!(f, "Error"),
            ConnectionState::ConnectionPending => write!(f, "Connection Pending")
        }
    }
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

/// Formats [`SessionState`] for human-readable display.
///
/// Used by [`ConnectionState`]'s [`Display`] implementation to compose the full
/// connection status string, and anywhere a `SessionState` is shown in the UI.
impl Display for SessionState {
    /// Writes a human-readable representation of the session state to `f`.
    ///
    /// # Variants
    /// - `LoginPending`      → `"Login"`
    /// - `LoggedOut`         → `"Logout"`
    /// - `LoggedIn`          → `"Log in"`
    /// - `LoginFailed(msg)`  → `"LoginFailed(<msg>)"` where `msg` is the server-supplied error
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SessionState::LoginPending     => write!(f, "Login"),
            SessionState::LoggedOut        => write!(f, "Logout"),
            SessionState::LoggedIn         => write!(f, "Log in"),
            SessionState::LoginFailed(msg) => write!(f, "LoginFailed({})", msg),
        }
    }
}
