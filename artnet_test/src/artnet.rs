use common::fixture;
use common::fixture::universe_count;
use std::io;
use std::net::UdpSocket;
use std::thread::sleep;
use std::time::{Duration, Instant};

const FIXTURES: usize = 1; //80;
const CHANNELS: usize = 512;
const TARGET: &str = "255.255.255.255:6454";
const FREQUENCY: u64 = 23;

pub fn artnet_loop() -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_broadcast(true)?;

    let mut sequence: u8 = 0;
    println!("Starting artnet");

    loop {
        let start = Instant::now();

        let universes = calculate_dmx_values();

        for (universe_index, data) in universes.iter().enumerate() {
            send_artnet(&socket, TARGET, data, universe_index as u16)?;
        }

        sequence = sequence.wrapping_add(1);

        let elapsed = start.elapsed();
        if elapsed < Duration::from_millis(FREQUENCY) {
            sleep(Duration::from_millis(FREQUENCY) - elapsed);
        }
    }
}

pub fn calculate_dmx_values() -> Vec<[u8; CHANNELS]> {
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
                    .get_mut(universe)
                    .unwrap()
                    .get_mut(*channel as usize)
                    .unwrap() = *value;
            });
    });

    universes
}

fn send_artnet_universe(
    socket: &UdpSocket,
    universe: u16,
    sequence: u8,
    dmx_data: &[u8; CHANNELS],
) -> std::io::Result<()> {
    let mut packet = Vec::with_capacity(18 + CHANNELS);

    // Art-Net ID
    packet.extend_from_slice(b"Art-Net\0");

    // OpCode (ArtDMX = 0x5000, little endian)
    packet.extend_from_slice(&0x5000u16.to_le_bytes());

    // Protocol Version (14)
    packet.extend_from_slice(&14u16.to_be_bytes());

    // Sequence + Physical
    packet.push(sequence);
    packet.push(0);

    // Universe (little endian)
    packet.extend_from_slice(&universe.to_le_bytes());

    // Length (big endian)
    packet.extend_from_slice(&(CHANNELS as u16).to_be_bytes());

    // DMX Daten
    packet.extend_from_slice(dmx_data);

    socket.send_to(&packet, TARGET)?;
    Ok(())
}

pub fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_broadcast(true)?;
    let target = "255.255.255.255:6454";

    let mut phase: f32 = 0.0;
    let speed: f32 = 0.05;

    loop {
        let start = Instant::now();

        let r = ((phase).sin() * 127.0 + 128.0) as u8;
        let g = ((phase + 2.094).sin() * 127.0 + 128.0) as u8; // +120°
        let b = ((phase + 4.188).sin() * 127.0 + 128.0) as u8; // +240°

        let mut dmx = [0u8; CHANNELS];

        for i in 0..FIXTURES {
            let base = i * 3;
            dmx[base] = r;
            dmx[base + 1] = g;
            dmx[base + 2] = b;
        }

        send_artnet(&socket, target, &mut dmx, 0)?;

        phase += speed;

        // ~44 FPS
        let frame_time = Duration::from_millis(23);
        let elapsed = start.elapsed();
        if elapsed < frame_time {
            sleep(frame_time - elapsed);
        }
    }
}

fn send_artnet(
    socket: &UdpSocket,
    target: &str,
    dmx: &[u8; 512],
    universe_id: u16,
) -> io::Result<()> {
    let mut packet = Vec::with_capacity(18 + CHANNELS);

    packet.extend_from_slice(b"Art-Net\0");
    packet.extend_from_slice(&[0x00, 0x50]); // ArtDMX
    packet.extend_from_slice(&[0x00, 14]); // Protocol version
    packet.extend_from_slice(&[0x00, 0x00]); // Sequence + Physical
    packet.extend_from_slice(&universe_id.to_le_bytes()); // Universe 0

    packet.extend_from_slice(&[(CHANNELS >> 8) as u8, (CHANNELS & 0xFF) as u8]);

    packet.extend_from_slice(dmx);

    socket.send_to(&packet, target)?;
    Ok(())
}
