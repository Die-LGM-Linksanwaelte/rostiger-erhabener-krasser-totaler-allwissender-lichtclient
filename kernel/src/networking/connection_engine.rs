use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::Sender;
use common::logging::LogLevel::*;
use common::networking::messages::{TcpServerMessage, UserRole, SessionID};
use common::networking::messages::TcpServerMessage::ShutdownAnnouncement;
use common::networking::subscription_objects::{SubscribeTopic, UpdateMode};
use common::r_log;

//Always change these two to the same type, if ConnectionID is changed to u32 for example, NEXT_CONNECTION_ID must be
// changed to AtomicU32
pub type ConnectionID = u64;
pub static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

pub static SERVER_STATE: LazyLock<RwLock<HashMap<SessionID, ClientSession>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub struct ClientSession {
    pub user_name: String,
    pub user_role: UserRole,

    pub active_connection: Option<(Sender<TcpServerMessage>, ConnectionID)>,
    pub subscriptions: Vec<(SubscribeTopic, UpdateMode)>,
    
}

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