use std::net::UdpSocket;
use std::io;
use std::thread::sleep;
use std::time::{Duration, Instant};
use Common::fixture;
use Common::fixture::universe_count;
use crate::artnet::ArtnetInterface;

const FIXTURES: usize = 1;//80;
pub const CHANNELS: usize = 512;
const TARGET: &str = "255.255.255.255:6454";
const FREQUENCY: u64 = 23;

pub trait DmxInterface {
    fn send_universe(&self, local_universe_index: u16, data: &[u8;CHANNELS]) -> Result<(), io::Error>;
}

pub fn artnet_loop() -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_broadcast(true)?;
    
    let artnet_interface = ArtnetInterface::new(socket, TARGET.to_string());

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

pub fn calculate_dmx_values() -> Vec<[u8;CHANNELS]>{
    let universe_count = universe_count();

    let mut universes = vec![[0u8; CHANNELS]; universe_count];

    let list = fixture::FIXTURE_LIST.read().unwrap();

    list.fixtures.iter().for_each(|(_, fixture)| {
        let universe = fixture.get_universe();

        fixture
            .get_channel_values()
            .iter()
            .for_each(|(channel, value)| {
                *universes
                    .get_mut(universe).unwrap()
                    .get_mut(*channel as usize).unwrap() = *value;
            });
    });

    universes
}
