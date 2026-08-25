//! # UDP Client / Listener Module
//!
//! This module handles high-speed, un-acknowledged UDP network reception.
//! It specifically listens for incoming Art-Net DMX protocol packets (OpOutput / OpDmx)
//! on the default port (6454), extracts DMX channel values for target universes, 
//! and dispatches universe data frames to the UI for live rendering.

use common::logging::LogLevel::*;
use common::r_log;
use eframe::egui;
use std::io;
use std::net::UdpSocket;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

/// Default UDP port defined by the Art-Net protocol specification.
const ARTNET_PORT: u16 = 6454;

/// Art-Net protocol magic header string bytes ("Art-Net\0").
const ARTNET_HEADER: &[u8; 8] = b"Art-Net\0";

/// Maximum number of DMX channels per universe according to DMX512 standard.
pub const MAX_CHANNEL: usize = 512;

/// Binds a UDP socket and spawns a background thread to listen for incoming Art-Net DMX packets.
///
/// Filters incoming UDP datagrams for valid Art-Net magic headers and OpDmx opcode (0x5000).
/// Extracted DMX frames are sent to the UI via `dmx_sender`, and an `egui::Context::request_repaint()`
/// is triggered to update the universe display immediately.
///
/// # Arguments
/// * `port` - Optional UDP port to bind to (defaults to Art-Net port 6454 if `None`).
/// * `dmx_sender` - Channel sender to transmit `(universe_id, dmx_data_array)` tuples to the controller.
/// * `ctx` - `egui::Context` reference used to request UI repaints upon packet arrival.
///
/// # Returns
/// `io::Result<()>` indicating whether the UDP socket binding was successful.
pub fn start_udp_listener(
    port: Option<u16>,
    dmx_sender: Sender<(u8, [u8; MAX_CHANNEL])>,
    ctx: egui::Context,
) -> io::Result<()> {
    let listen_port = port.unwrap_or(ARTNET_PORT);
    let addr = format!("0.0.0.0:{}", listen_port);

    let socket = UdpSocket::bind(&addr)?;
    r_log!(Info, "UDP-Listener on adress {} startet!", addr);

    socket.set_read_timeout(Some(Duration::from_millis(100)))?;

    thread::spawn(move || {
        let mut buf = [0; 1024];

        loop {
            match socket.recv_from(&mut buf) {
                Ok((bytes_count, src_addr)) => {
                    let data = &buf[..bytes_count];

                    if is_artnet_packet(data) {
                        if data.len() >= 18 && u16::from_le_bytes([data[8], data[9]]) == 0x5000 {
                            let universe_id = data[14];
                            let dmx_length = u16::from_be_bytes([data[16], data[17]]) as usize;
                            let dmx_start_index = 18;

                            if data.len() >= dmx_start_index + dmx_length {
                                let mut dmx_data_array = [0; MAX_CHANNEL];
                                let actual_dmx_len = dmx_length.min(MAX_CHANNEL);
                                dmx_data_array[..actual_dmx_len].copy_from_slice(
                                    &data[dmx_start_index..dmx_start_index + actual_dmx_len],
                                );

                                if let Err(e) = dmx_sender.send((universe_id, dmx_data_array)) {
                                    r_log!(Info, "DMX channel closed, stopping UDP listener: {}", e);
                                    break;
                                } else {
                                    ctx.request_repaint();
                                }
                            } else {
                                r_log!(Warning, "Uncompleted ArtNet-Paket from {}", src_addr);
                            }
                        }
                    } else {
                        handle_generic_packet(data, src_addr);
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    r_log!(Error, "Error receiving UDP-Packets: {}", e);
                    break;
                }
            }
        }
    });

    Ok(())
}

/// Helper function to check if a received byte slice begins with the standard Art-Net header.
///
/// # Arguments
/// * `data` - Raw incoming packet byte buffer.
///
/// # Returns
/// `true` if the packet is at least 10 bytes long and starts with `"Art-Net\0"`.
fn is_artnet_packet(data: &[u8]) -> bool {
    data.len() >= 10 && &data[0..8] == ARTNET_HEADER
}

/// Fallback handler for non-Art-Net UDP datagrams received on the socket.
///
/// # Arguments
/// * `data` - Raw packet byte payload.
/// * `src` - Sender socket address.
fn handle_generic_packet(data: &[u8], src: std::net::SocketAddr) {
    r_log!(Info, "Received other UDP-Packet {}: {} Bytes", src, data.len());
}
