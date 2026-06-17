use crate::artnet::ArtnetInterface;
use common::fixture::{MAX_CHANNEL, calculate_dmx_values};
use std::io;
use std::sync::{LazyLock, RwLock};
use std::net::UdpSocket;
use std::thread::sleep;
use std::time::{Duration, Instant};
use crate::enttec_dmx_usb_pro::EnttecDmxPro;

const TARGET: &str = "255.255.255.255:6454";
const FREQUENCY: u64 = 23;

pub static ENTEC: LazyLock<RwLock<Option<EnttecDmxPro>>> =
    LazyLock::new(|| RwLock::new(None));

pub static INTERFACES: LazyLock<RwLock<Vec<Box<dyn DmxInterface + Send + Sync>>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));


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

    println!("Starting artnet");

    loop {
        let start = Instant::now();

        let universes = calculate_dmx_values();

        for (universe_index, data) in universes.iter().enumerate() {
            if let Some(entec) = ENTEC.read().expect("Failed to get entec lock").as_ref() {
                if universe_index <= 2 {
                    if let Err(e) = entec.send_universe(universe_index as u16, data) {
                        match e.kind() {
                            io::ErrorKind::BrokenPipe => {
                                println!("Broken pipe");
                                let mut entec = ENTEC.write().expect("Failed to write entec lock");
                                *entec = None;
                            }

                            _ => return Err(e),
                        }
                    };
                }
            }
            artnet_interface.send_universe(universe_index as u16, data)?;
        }

        let elapsed = start.elapsed();
        if elapsed < Duration::from_millis(FREQUENCY) {
            sleep(Duration::from_millis(FREQUENCY) - elapsed);
        }
    }
}

pub fn setup_entec(port: &str) {
    let mut entec = ENTEC.write().expect("Failed to write entec lock");

    *entec = Some(EnttecDmxPro::new(port).expect(&format!("Failed to setup DMX port: {}", port)));
}
