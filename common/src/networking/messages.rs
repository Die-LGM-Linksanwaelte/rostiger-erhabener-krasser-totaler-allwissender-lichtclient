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
        user_id: u8,
        user_name: String,
        user_role: UserRole,
    },
    
    Disconnect,

    Subscribe {
        topic: SubscribeTopic,
        update_mode: UpdateMode
    },

    Unsubscribe {
        topic: SubscribeTopic,
    }
}




//This is only Temporarily here, will be moved to a different location once this location is programmed
#[derive(Serialize, Deserialize, Debug)]
pub enum UserRole {
    Programmer,
    BlindProgrammer,
    Showrunner,
    Interface
}