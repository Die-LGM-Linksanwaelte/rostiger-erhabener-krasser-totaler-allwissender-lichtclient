use std::fmt;
use std::fmt::Formatter;
use serde::{Deserialize, Serialize};
use crate::logging::LogLevel;
use crate::cli_actions::{CliAction, CliActionResponse};
use crate::networking::subscription_objects::{SubscribeTopic, UpdateMode, TopicPayload};

/// Unique identifier for an active client session.
pub type SessionID = u64;

/// Initial payload sent by a client upon connecting to verify compatibility.
#[derive(Serialize, Deserialize, Debug)]
pub struct HandshakeRequest {
    /// Must always be exactly "REKTAL" to identify the protocol.
    pub magic_string: String,
    /// A hash verifying the client and server share the exact same network message definitions.
    pub protocol_hash: String,
    /// The human-readable version string of the client software.
    pub client_version: String,
}

/// The kernel's response to a client's handshake request.
#[derive(Serialize, Deserialize, Debug)]
pub enum HandshakeResponse {
    /// Handshake successful; protocol and versions match.
    Ok,
    /// Handshake failed due to a protocol hash mismatch.
    Mismatch { server_version: String },
}

/// Represents all valid messages dispatched from a client to the kernel over an active TCP connection.
#[derive(Serialize, Deserialize, Debug)]
pub enum TcpClientMessage {
    /// Request to authenticate a new session.
    Login {
        password: String,
        user_name: String,
        user_role: UserRole,
    },

    /// Request to safely terminate the current session.
    Logout,

    /// Request to resume a previous session using an existing token.
    Relogin {
        user_id: SessionID,
        clear_subscriptions: bool,
    },

    /// Request to listen for state changes on a specific topic.
    Subscribe {
        topic: SubscribeTopic,
        update_mode: UpdateMode
    },

    /// Request to stop receiving updates for a specific topic.
    Unsubscribe {
        topic: SubscribeTopic,
    },

    /// Submits a raw CLI string for the server to parse and execute.
    ExecuteCommand {
        command: String,
        response_id: u32
    },

    /// Submits a pre-parsed, structured CLI action directly.
    ExecuteImplicitCommand {
        command: CliAction,
        response_id: u32
    },

    /// Requests exclusive editing rights for a specific engine resource.
    RequestEdit(EditableResource),

    /// Submits modifications for an exclusively locked resource.
    SubmitEdit {
        resource: EditableResource,
        new_data: Vec<u8>
    },

    /// Drops the Lock on an exclusively locked resource, giving other Clients the chance to lock it.
    DropEditLock(EditableResource),
}

/// Represents all valid messages dispatched from the kernel back to a connected client.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TcpServerMessage {
    /// Emitted when a client attempts an action before authenticating.
    Unauthenticated,

    /// Indicates a successful login, providing the session token (for reconnecting later).
    LoginOk {token: SessionID},
    /// Indicates a failed login attempt with the given reason.
    LoginFailed {reason: String},

    /// Indicates a successful session resumption.
    ReloginOk {token: SessionID},
    /// Indicates a failed session resumption.
    ReloginFailed {reason: String},

    /// Confirms the session has been terminated.
    LogoutOk,

    /// Informs the client that the server forcibly closed the connection.
    Kicked {reason: String},

    /// Contains the text and log level output of a standard string command execution.
    CommandOutput {
        answer: (LogLevel, String),
        response_id: u32
    },
    /// Contains the structured response of an implicit (pre-parsed) command execution.
    ImplicitCommandOutput {
        answer: CliActionResponse,
        response_id: u32
    },

    /// Broadcasts new state data for a subscribed topic.
    TopicUpdate {
        data: TopicPayload,
    },

    /// Grants the client exclusive edit access to a resource, providing its current state.
    EditGranted {
        resource: EditableResource,
        current_data: Vec<u8>
    },
    /// Denies an edit request because the resource is already locked or unavailable.
    EditDenied {
        resource: EditableResource,
        reason: String
    },
    /// Acknowledges, that the Resource-Lock has been dropped, and can be picked up by other clients
    DropEditAck(EditableResource),

    /// Notifies all clients that the server is gracefully shutting down.
    ShutdownAnnouncement,
}



//Dummy Enum until I implement it right
/// Identifies a specific engine resource that can be locked for exclusive editing. Has currently no functionality, will
/// be implemented later.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EditableResource {
    Cuelist,
}

//This is only Temporarily here, will be moved to a different location once this location is programmed
//Also, Interface may not belong here, that's why ist commented out, since it should be handled differently than the GUIs
/// Defines the permission level and operational scope of a connected user.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
pub enum UserRole {
    /// Full access to edit the show and trigger outputs.
    Programmer,
    /// Access to edit the show without affecting live outputs (blind programming).
    BlindProgrammer,
    /// Access to trigger playbacks and run the show, but restricted from editing structural data.
    Showrunner,
//    Interface
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let text = match self {
            UserRole::Programmer => "programmer",
            UserRole::BlindProgrammer => "blind programmer",
            UserRole::Showrunner => "showrunner",
        };

        write!(f, "{text}")
    }
}

/// Retrieves the compile-time protocol hash to verify network message compatibility.
pub fn get_protocol_version() -> String {
    env!("PROTOCOL_HASH").to_string()
}