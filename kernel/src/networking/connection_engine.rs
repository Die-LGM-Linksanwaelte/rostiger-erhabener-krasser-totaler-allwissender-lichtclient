use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::Sender;
use common::logging::LogLevel::*;
use common::networking::messages::{TcpServerMessage, UserRole, SessionID};
use common::networking::messages::TcpServerMessage::ShutdownAnnouncement;
use common::networking::subscription_objects::{SubscribeTopic, UpdateMode};
use common::r_log;

/// Unique identifier for an active physical TCP connection.
///
/// **Note:** If this type is changed (e.g., to `u32`), [`NEXT_CONNECTION_ID`]
/// must be updated to the corresponding atomic type (e.g., `AtomicU32`).
pub(super) type ConnectionID = u64;
/// Thread-safe atomic counter used to generate unique [`ConnectionID`]s for incoming clients.
pub(super) static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

/// The central, globally accessible repository of all authenticated client sessions.
/// Maps a unique [`SessionID`] to its corresponding [`ClientSession`].
pub(super) static SERVER_STATE: LazyLock<RwLock<HashMap<SessionID, ClientSession>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Represents an authenticated user session within the server.
///
/// A session persists even if the physical connection drops, allowing clients to
/// seamlessly reconnect and resume their subscriptions and privileges without logging in again.
pub(super) struct ClientSession {
    /// The display name of the authenticated user.
    pub _user_name: String,
    /// The permission level and operational scope assigned to this user.
    pub _user_role: UserRole,

    /// The active communication channel to the client, if currently connected.
    /// Stores the `mpsc::Sender` for dispatching network messages and the physical [`ConnectionID`].
    pub active_connection: Option<(Sender<TcpServerMessage>, ConnectionID)>,
    /// A list of engine state topics this session is currently subscribed to.
    pub subscriptions: Vec<(SubscribeTopic, UpdateMode)>,
    
}

/// Broadcasts a graceful shutdown announcement to all currently connected clients.
///
/// Iterates through the global [`SERVER_STATE`] and attempts to dispatch a
/// `ShutdownAnnouncement` message via every active connection channel.
pub fn announce_shutdown() {
    let server_state = SERVER_STATE.read().unwrap();
    for connection in server_state.values() {
        if let Some((channel,connection_id)) = connection.active_connection.as_ref() {
            if let Err(e) = channel.send(ShutdownAnnouncement) {
                r_log!(Error,"[Conn {}] Error sending Shutdown-Announcement to channel: {}", connection_id, e);
            }
        }
    }
}