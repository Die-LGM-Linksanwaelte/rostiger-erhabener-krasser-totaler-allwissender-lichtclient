use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use std::sync::atomic::AtomicU64;
use crate::networking::messages::{TcpServerMessage, UserRole};
use std::sync::mpsc::Sender;

pub type SessionID = u64;

//Always change these two to the same type, if ConnectionID is changed to u32 for example, NEXT_CONNECTION_ID must be
// changed to AtomicU32
pub type ConnectionID = u64;
pub static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

pub static SERVER_STATE: LazyLock<RwLock<HashMap<SessionID, ClientSession>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub struct ClientSession {
    pub user_name: String,
    pub user_role: UserRole,

    pub active_connection: Option<(Sender<TcpServerMessage>, u64)>
}