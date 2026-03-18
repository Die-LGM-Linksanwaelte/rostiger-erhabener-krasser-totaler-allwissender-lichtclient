use std::net::UdpSocket;
use std::io;
use std::thread::sleep;
use std::time::{Duration, Instant};
use Common::fixture::{MAX_CHANNEL, calculate_dmx_values};
use crate::artnet::ArtnetInterface;

const TARGET: &str = "255.255.255.255:6454";
const FREQUENCY: u64 = 23;

pub trait DmxInterface {
    fn send_universe(&self, local_universe_index: u16, data: &[u8;MAX_CHANNEL as usize]) -> Result<(), io::Error>;
}

pub fn dmx_output_loop() -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_broadcast(true)?;
    
    let artnet_interface: Box<dyn DmxInterface> = Box::new(ArtnetInterface::new(socket, TARGET.to_string()));

    println!("Starting artnet");


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
