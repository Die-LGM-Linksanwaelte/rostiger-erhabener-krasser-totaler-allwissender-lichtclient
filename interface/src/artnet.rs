use crate::interfaces::DmxInterface;
use common::fixture::MAX_CHANNEL;
use std::io;
use std::net::UdpSocket;

/// Interface for outputting to artnet. Uses no artnet-library, works only with UDP. Implements ['DmxInterface']
pub(crate) struct ArtnetInterface {
    socket: UdpSocket,
    target: String,
    sequence: u8,
}

impl ArtnetInterface {
    /// Constructor for ArtnetInterface.
    /// # Arguments
    ///
    /// * 'socket' - UDP-socket used for outputting artnet. Doesn't need to set_broadcast to true, because it already
    ///             happens in this method
    /// * 'target' - The target IP-address the artnet-packages get sent to. Warning: If this String is an valid
    ///             IP-address doesn't get checked until sending the packages. May be changed at a later point in
    ///             Development. TODO
    pub fn new(socket: UdpSocket, target: String) -> Self {
        socket.set_broadcast(true).unwrap();
        Self {
            socket,
            target,
            sequence: 0,
        }
    }
}

impl DmxInterface for ArtnetInterface {
    fn send_universe(
        &self,
        local_universe_index: u16,
        data: &[u8; MAX_CHANNEL as usize],
    ) -> io::Result<()> {
        let mut packet = Vec::with_capacity(18 + MAX_CHANNEL as usize);

        // Art-Net ID
        packet.extend_from_slice(b"Art-Net\0");

        // OpCode (ArtDMX = 0x5000, little endian)
        packet.extend_from_slice(&0x5000u16.to_le_bytes());

        // Protocol Version (14)
        packet.extend_from_slice(&14u16.to_be_bytes());

        // Sequence + Physical
        packet.push(self.sequence);
        packet.push(0);

        // Universe (little endian)
        packet.extend_from_slice(&local_universe_index.to_le_bytes());

        // Length (big endian)
        packet.extend_from_slice(&MAX_CHANNEL.to_be_bytes());

        // DMX Data
        packet.extend_from_slice(data);

        self.socket.send_to(&packet, self.target.clone())?;
        Ok(())
    }
}
