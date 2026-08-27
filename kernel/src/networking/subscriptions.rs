use std::sync::{LazyLock, RwLock};
use common::networking::messages::{TcpServerMessage, SessionID};
use common::networking::subscription_objects::{DMXConfigForClientState, SubscribeTopic, TopicPayload, UpdateMode};
use common::networking::subscription_objects::UpdateMode::*;
use crate::networking::connection_engine::SERVER_STATE;

/// A globally accessible cache that stores the most recent state of all subscribable topics.
/// This buffer serves a dual purpose: it provides immediate data to newly connected subscribers
/// upon request, and it acts as the primary data source for background workers managing
/// continuous-mode or other subscription streams.
struct ContinuousBuffer {
    /// The cached visual representation of the current DMX patch and allocations.
    dmx_config: DMXConfigForClientState,
}

impl ContinuousBuffer {
    /// Initializes a new, empty continuous buffer.
    fn new() -> Self {
        Self {
            dmx_config: vec![vec![]]
        }
    }
}

/// Thread-safe static instance of the [`ContinuousBuffer`].
static CONTINUOUS_BUFFER: LazyLock<RwLock<ContinuousBuffer>> =
LazyLock::new(|| RwLock::new(ContinuousBuffer::new()));


/// Registers a new topic subscription for a specific client session.
///
/// Upon successful registration, this function immediately fetches the latest known state
/// from the [`CONTINUOUS_BUFFER`] and dispatches it to the client. This guarantees
/// the client interface is instantly populated with valid data.
///
/// # Arguments
///
/// * `token`       - The unique session identifier of the requesting client.
/// * `topic`       - The specific engine state topic the client wishes to monitor.
/// * `update_mode` - The frequency mode (e.g., continuous or on-change) for this subscription.
pub fn add_subscription(token: &SessionID, topic: &SubscribeTopic, update_mode: &UpdateMode) {
    let mut server_state = SERVER_STATE.write().unwrap();
    let user_data = server_state.get_mut(&token);

    if let Some(user_data) = user_data {
        user_data.subscriptions.push((topic.clone(), update_mode.clone()));

        if let Some((sender, _)) = &user_data.active_connection {

            let buffer = &CONTINUOUS_BUFFER.read().unwrap();
            let payload = match topic {
                SubscribeTopic::DMXConfiguration => {
                    TopicPayload::DMXConfiguration(buffer.dmx_config.clone())
                }
            };

            sender.send(
                TcpServerMessage::TopicUpdate { data: payload }
            ).unwrap();
        }
    }
}

/// Entry point for engine-side DMX configuration changes.
///
/// When the internal DMX allocation changes, this function updates the global cache
/// and broadcasts the new state to all clients actively listening for changes.
///
/// # Arguments
///
/// * `data` - The newly computed 2D structure of the client-facing DMX configuration.
pub fn on_dmx_config_update(data: DMXConfigForClientState) {

    {
        CONTINUOUS_BUFFER.write().unwrap().dmx_config = data.clone();
    }

    let data = TopicPayload::DMXConfiguration(data);

    send_updates(data, OnChange);
}

//TODO
/// Internal helper function that broadcasts a state payload to all matching client sessions.
///
/// Iterates through the global `SERVER_STATE` and transmits the update to any session
/// that holds an active TCP connection and matches both the topic and the requested update mode.
/// Currently, all Update-Modes are handled as OnChange
///
/// # Arguments
///
/// * `payload`     - The actual data payload to be broadcast.
/// * `update_mode` - The update condition (e.g., `OnChange`) that triggered this broadcast.
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
