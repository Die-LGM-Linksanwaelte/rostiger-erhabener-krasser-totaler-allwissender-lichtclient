use std::fmt;
use std::fmt::Formatter;
use serde::{Deserialize, Serialize};
use crate::fixture::PropertyType;

pub type DMXConfigForClientState = Vec<Vec<DMXConfigurationForClient>>;

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

