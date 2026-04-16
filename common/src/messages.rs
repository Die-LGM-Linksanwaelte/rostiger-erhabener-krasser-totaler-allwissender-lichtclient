use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum TcpMessage {
    // TODO: alle messages, die über tcp ausgetauscht werden, also z.b. addDevice, etc.
    // über tcp nur das, was zeitunkritisch ist oder garantiert sicher ankommen muss
    //
}
