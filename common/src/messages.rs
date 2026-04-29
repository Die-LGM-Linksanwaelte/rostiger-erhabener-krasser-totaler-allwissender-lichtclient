use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum TcpMessage {
    // TODO: alle messages, die über tcp ausgetauscht werden, also z.b. addDevice, etc.
    // über tcp nur das, was zeitunkritisch ist oder garantiert sicher ankommen muss
    CreateFixture {
        fixture_type: FixtureType,
    },

    CreateDevice {
        start_channel: u16,
        universe: usize,
        name: String,
    },
    SetDeviceAdress {
        new_start_channel: u16,
        new_universe: usize,
    },
    SetDeviceFixture,
    DeleteDevice,
    SetDeviceProperty {
        property: SimplePropertyType,
        value: Channel,
    },

    LoadProject,
    SaveProject,
}
