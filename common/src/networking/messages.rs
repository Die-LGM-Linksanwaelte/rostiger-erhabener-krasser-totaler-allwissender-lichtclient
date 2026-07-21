use serde::{Deserialize, Serialize};

//TODO: Add better topics when we know wich topics are subscribeworthy
#[derive(Serialize, Deserialize, Debug)]
pub enum SubscribeTopic {
    Universes,
    FixturePositions,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UpdateMode {
    OnChange,
    Continuous,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TcpClientMessage {
    Connect {
        password: String,
        user_name: String,
        user_role: UserRole,
    },

    Disconnect,

    Reconnect {
        password: String,
        user_id: u64,
        clear_subscriptions: bool,
    },

    Subscribe {
        topic: SubscribeTopic,
        update_mode: UpdateMode,
    },

    Unsubscribe {
        topic: SubscribeTopic,
    },

    ExecuteCommand(String),

    RequestEdit(EditableResource),

    SubmitEdit {
        resource: EditableResource,
        new_data: Option<Vec<u8>>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TcpServerMessage {
    AssignUserID(u64),

    CommandOutput(Result<String, String>),

    TopicUpdate {
        topic: SubscribeTopic,
        data: Vec<u8>,
    },

    EditGranted {
        resource: EditableResource,
        current_data: Vec<u8>,
    },

    EditDeniend {
        resource: EditableResource,
        reason: String,
    },
}

//Dummy Enum until i implement it right
#[derive(Serialize, Deserialize, Debug)]
pub enum EditableResource {
    Cuelist,
}

//This is only Temporarily here, will be moved to a different location once this location is programmed
//Also, Interface may not belong here, thats why ist commented out, since it should be handled differently than the GUIs
#[derive(Serialize, Deserialize, Debug)]
pub enum UserRole {
    Programmer,
    BlindProgrammer,
    Showrunner,
    //    Interface
}
