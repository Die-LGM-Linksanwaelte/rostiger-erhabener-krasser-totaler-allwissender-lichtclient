use crate::artnet::ArtnetInterface;
use common::fixture::{MAX_CHANNEL, calculate_dmx_values, Fixture};
use std::io;
use std::net::UdpSocket;
use std::sync::mpsc::Receiver;
use std::thread::sleep;
use std::time::{Duration, Instant};
use common::logging::LogLevel::*;
use common::r_log;

const TARGET: &str = "255.255.255.255:6454";
const FREQUENCY: u64 = 23;

//TODO
/// Defines the baseline behavior for any hardware or network DMX output interface.
///
/// **Note:** Currently, the system hardcodes ArtNet output. Future refactoring
/// will allow modular instantiation of multiple interface types via this trait.
pub trait DmxInterface {
    /// Dispatches a single DMX universe to the connected lighting fixtures.
    ///
    /// # Arguments
    ///
    /// * `local_universe_index` - The 0-based index of the universe to output on (useful for multi-universe nodes).
    /// * `data` - A full 512-byte array representing the computed DMX channel values for this universe.
    fn send_universe(
        &self,
        local_universe_index: u16,
        data: &[u8; MAX_CHANNEL as usize],
    ) -> Result<(), io::Error>;
}

//TODO
/// Initializes the output interface and continuously streams DMX data on a dedicated thread.
///
/// This loop non-blockingly consumes the latest engine state from the provided MPSC receiver,
/// calculates the raw DMX universe buffers, and dispatches them via the configured interfaces.
/// It automatically throttles its execution to match the target frame rate defined by [`FREQUENCY`].
///
/// # Arguments
///
/// * `data_receiver` - A channel receiver providing tuple updates containing the current
///   total universe count and the latest snapshot of all active fixtures.
pub fn dmx_output_loop(data_receiver: Receiver<(usize,Vec<Fixture>)>) -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_broadcast(true)?;

    let artnet_interface: Box<dyn DmxInterface> =
        Box::new(ArtnetInterface::new(socket, TARGET.to_string()));

    r_log!(Info,"Starting artnet");

    let mut universe_count: usize = 0;
    let mut fixtures: Vec<Fixture> = vec![];

    loop {
        let start = Instant::now();

        while let Ok((new_count, new_fixtures)) = data_receiver.try_recv() {
            universe_count = new_count;
            fixtures = new_fixtures;
        }


        let universes = calculate_dmx_values(universe_count, &fixtures);

        for (universe_index, data) in universes.iter().enumerate() {
            artnet_interface.send_universe(universe_index as u16, data)?;
        }

        let elapsed = start.elapsed();
        if elapsed < Duration::from_millis(FREQUENCY) {
            sleep(Duration::from_millis(FREQUENCY) - elapsed);
        }
    }
}
