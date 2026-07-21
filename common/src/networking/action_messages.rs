use crate::fixture::FixtureType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum TcpMessageAction {
    // TODO: alle aktionen, die über tcp auf den Server asugeführt werden sollen, also z.b. addDevice, etc.
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
        //property: ,
        //value: ,
    },

    LoadProject,
    SaveProject,
}
