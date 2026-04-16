use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum TcpMessage {
    // TODO: alle messages, die über tcp ausgetauscht werden, also z.b. addDevice, etc.
    // über tcp nur das, was zeitunkritisch ist oder garantiert sicher ankommen muss
    createFixture {
        fixture_type: FixtureType,
    },

    createDevice {
        start_channel: u16,
        universe: usize,
        name: String,
    },
    setDeviceAdress {
        new_start_channel: u16,
        new_universe: usize,
    },
    setDeviceFixture,
    deleteDevice,
    setDeviceProperty {
        property: ,
        value: ,
    },

    loadProject,
    saveProject,
}
