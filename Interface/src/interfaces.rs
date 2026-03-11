use std::net::UdpSocket;
use std::io;
use std::thread::sleep;
use std::time::{Duration, Instant};
use Common::fixture;
use Common::fixture::{universe_count, ChannelError, MAX_CHANNEL};
use crate::artnet::ArtnetInterface;

const TARGET: &str = "255.255.255.255:6454";
const FREQUENCY: u64 = 23;

pub trait DmxInterface {
    fn send_universe(&self, local_universe_index: u16, data: &[u8;MAX_CHANNEL as usize]) -> Result<(), io::Error>;
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

pub fn calculate_dmx_values() -> Vec<[u8;MAX_CHANNEL as usize]>{
    let universe_count = universe_count();

    let mut output = vec![[0u8; MAX_CHANNEL as usize]; universe_count];

    let list = fixture::FIXTURE_LIST.read().unwrap();

    list.fixtures.iter().for_each(|(_, fixture)| {
        let universe_number = fixture.get_universe();
        let fixture_type = fixture.get_fixture_type();
        let fixture_name = fixture.get_name();

        if universe_number < universe_count {
            fixture
                .get_channel_values()
                .iter()
                .for_each(|(channel, value)| {
                    *output.get_mut(universe_number).unwrap()
                        .get_mut(*channel as usize)
                            .ok_or(ChannelError::ChannelOutOfRange).expect(
                            &format!("Fixture \"{}\" of type {} has a channel that is out of bounds",
                                     fixture_name, fixture_type
                            ))
                            = *value;

                });
        }

    });

    output
}
