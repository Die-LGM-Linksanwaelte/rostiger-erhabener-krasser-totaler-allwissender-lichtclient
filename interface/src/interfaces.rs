use crate::artnet::ArtnetInterface;
use common::fixture::{MAX_CHANNEL, calculate_dmx_values};
use common::logging::LogLevel::*;
use common::r_log;
use std::io;
use std::net::UdpSocket;
use std::thread::sleep;
use std::time::{Duration, Instant};

const TARGET: &str = "255.255.255.255:6454";
const FREQUENCY: u64 = 23;

/// Trait that should be implemented by all interfaces. For now, there is no possibility to modularly create a
/// DmxInterface. TODO
pub trait DmxInterface {
    /// Sends universes to fixtures.
    ///
    /// # Arguments
    ///
    /// * 'local_universe_index' - If an interface can output more than one universe, this parameter is set to the
    ///                            universe we want to output on (0-indexed)
    /// * 'data' - An Array with all the DMX-values of the universe we want to output
    fn send_universe(
        &self,
        local_universe_index: u16,
        data: &[u8; MAX_CHANNEL as usize],
    ) -> Result<(), io::Error>;
}

/// Initiates the interfaces, then calculates the DMX-values and outputs them to the interfaces. For now, this function
/// outputs the DMX-values all to artnet, but this should later be changed to allow all interfaces, that implement the
/// trait ['DmxInterface']. TODO
pub fn dmx_output_loop() -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_broadcast(true)?;

    let artnet_interface: Box<dyn DmxInterface> =
        Box::new(ArtnetInterface::new(socket, TARGET.to_string()));

    r_log!(Info, "Starting artnet");

    loop {
        let start = Instant::now();

        let universes = calculate_dmx_values();

        for (universe_index, data) in universes.iter().enumerate() {
            artnet_interface.send_universe(universe_index as u16, data)?;
        }

        let elapsed = start.elapsed();
        if elapsed < Duration::from_millis(FREQUENCY) {
            sleep(Duration::from_millis(FREQUENCY) - elapsed);
        }
    }
}
