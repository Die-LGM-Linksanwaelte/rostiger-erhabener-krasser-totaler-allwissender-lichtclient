use std::fmt;
use std::fmt::Formatter;
use std::sync::{LazyLock, RwLock};
use serde::{Deserialize, Serialize};
use crate::fixture::PropertyType;
use crate::networking::connection_engine::SERVER_STATE;
use crate::networking::messages::TcpServerMessage;
use crate::networking::messages::UpdateMode::OnChange;

pub type DMXConfigForClientState = Vec<Vec<DMXConfigurationForClient>>;

struct ContinuousBuffer {
    dmxconfig: DMXConfigForClientState,
}

impl ContinuousBuffer {
    fn new() -> Self {
        Self {
            dmxconfig: vec![vec![]]
        }
    }
}

static CONTINUOUS_BUFFER: LazyLock<RwLock<ContinuousBuffer>> =
LazyLock::new(|| RwLock::new(ContinuousBuffer::new()));

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum SubscribeTopic {
    DMXConfiguration,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TopicPayload {
    DMXConfiguration(DMXConfigForClientState),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum UpdateMode {
    OnChange,
    Continuous,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DMXConfigurationForClient {
    Empty,
    Reserved{
        fixture_name: String,
        property_type: PropertyType,
        fine_degree: usize,
        fixture_type_hash: u8,
    },
}

pub fn on_dmx_config_update(data: DMXConfigForClientState) {

    {
        let mut buffer = &CONTINUOUS_BUFFER.write().unwrap().dmxconfig;
        buffer = &data.clone();
    }

    let data = TopicPayload::DMXConfiguration(data);

    send_updates(data, OnChange);
}

fn send_updates(payload: TopicPayload, update_mode: UpdateMode) {
    let server_state = SERVER_STATE.read().unwrap();

    let topic = payload.get_topic();

    let data = TcpServerMessage::TopicUpdate {
        data: payload
    };

    for session in server_state.values() {
        if session.subscriptions.contains(&(topic.clone(), update_mode.clone())) {
            if let Some((ref sender,_)) = session.active_connection {
                sender.send(data.clone()).unwrap();
            }
        }
    }
}

impl TopicPayload {
    pub fn get_topic(&self) -> SubscribeTopic {
        match self {
            TopicPayload::DMXConfiguration(..) => SubscribeTopic::DMXConfiguration,
        }
    }
}

impl fmt::Display for SubscribeTopic {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SubscribeTopic::DMXConfiguration => f.write_str("DMXConfiguration"),
        }
    }
}
