use std::fmt;
use std::fmt::Formatter;
use serde::{Deserialize, Serialize};
use crate::logging::LogLevel;
use crate::cli_actions::CliAction;
use crate::networking::subscription_objects::{SubscribeTopic, UpdateMode, TopicPayload};

pub type SessionID = u64;

#[derive(Serialize, Deserialize, Debug)]
pub struct HandshakeRequest {
    pub magic_string: String, // Must always be "REKTAL" 
    pub protocol_hash: String,
    pub client_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum HandshakeResponse {
    Ok,
    Mismatch { server_version: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TcpClientMessage {
    Login {
        password: String,
        user_name: String,
        user_role: UserRole,
    },

    Logout,

    Relogin {
        user_id: SessionID,
        clear_subscriptions: bool,
    },

    Subscribe {
        topic: SubscribeTopic,
        update_mode: UpdateMode
    },

    Unsubscribe {
        topic: SubscribeTopic,
    },

    ExecuteCommand {
        command: String,
        response_id: u32
    },

    ExecuteImplicitCommand {
        command: CliAction,
        response_id: u32
    },

    RequestEdit(EditableResource),

    SubmitEdit {
        resource: EditableResource,
        new_data: Option<Vec<u8>>
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TcpServerMessage {
    //Answer when sending anything without being logged in
    Unauthenticated,
    
    //Login answers
    LoginOk {token: SessionID},
    LoginFailed {reason: String},

    //Relogin answers
    ReloginOk {token: SessionID},
    ReloginFailed {reason: String},

    //Logout answer
    LogoutOk,

    Kicked {reason: String},

    CommandOutput {
        answer: (LogLevel, String),
        response_id: u32
    },

    TopicUpdate {
        data: TopicPayload,
    },

    EditGranted {
        resource: EditableResource,
        current_data: Vec<u8>
    },

    EditDenied {
        resource: EditableResource,
        reason: String
    },
}



//Dummy Enum until I implement it right
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EditableResource {
    Cuelist,
}

//This is only Temporarily here, will be moved to a different location once this location is programmed
//Also, Interface may not belong here, that's why ist commented out, since it should be handled differently than the GUIs
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
pub enum UserRole {
    Programmer,
    BlindProgrammer,
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

pub fn get_protocol_version() -> String {
    env!("PROTOCOL_HASH").to_string()
}