use std::net::UdpSocket;
use std::thread::sleep;
use std::time::{Duration, Instant};

const FIXTURES: usize = 80;
const CHANNELS: usize = FIXTURES * 3;

fn main() -> std::io::Result<()> {
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

        let mut packet = Vec::with_capacity(18 + CHANNELS);

        packet.extend_from_slice(b"Art-Net\0");
        packet.extend_from_slice(&[0x00, 0x50]); // ArtDMX
        packet.extend_from_slice(&[0x00, 14]);   // Protocol version
        packet.extend_from_slice(&[0x00, 0x00]); // Sequence + Physical
        packet.extend_from_slice(&[0x00, 0x00]); // Universe 0

        packet.extend_from_slice(&[
            (CHANNELS >> 8) as u8,
            (CHANNELS & 0xFF) as u8,
        ]);

        packet.extend_from_slice(&dmx);

        socket.send_to(&packet, target)?;

        phase += speed;

        // ~44 FPS
        let frame_time = Duration::from_millis(23);
        let elapsed = start.elapsed();
        if elapsed < frame_time {
            sleep(frame_time - elapsed);
        }
    }
}
