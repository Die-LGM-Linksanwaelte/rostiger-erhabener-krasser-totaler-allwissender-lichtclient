use eframe::egui;
use std::io;
use std::net::UdpSocket;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration; // Hinzugefügt, um den Context zu kennen

const ARTNET_PORT: u16 = 6454;
const ARTNET_HEADER: &[u8; 8] = b"Art-Net\0";
pub const MAX_CHANNEL: usize = 512;

pub fn start_udp_listener(
    port: Option<u16>,
    dmx_sender: Sender<(u8, [u8; MAX_CHANNEL])>,
    ctx: egui::Context, // Nimmt jetzt direkt den Egui-Context an!
) -> io::Result<()> {
    let listen_port = port.unwrap_or(ARTNET_PORT);
    let addr = format!("0.0.0.0:{}", listen_port);

    let socket = UdpSocket::bind(&addr)?;
    println!("UDP-Listener on adress {} startet!", addr);

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

                                // Daten erfolgreich an den UI Thread gesendet...
                                if let Err(e) = dmx_sender.send((universe_id, dmx_data_array)) {
                                    eprintln!("Error sending DMX-Data: {}", e);
                                    break; // Thread beenden, wenn der Receiver weg ist
                                } else {
                                    // DER ENTSCHEIDENDE BEFEHL:
                                    // Wecke die GUI SOFORT auf! Egal wo die Maus ist.
                                    ctx.request_repaint();
                                }
                            } else {
                                eprintln!("Uncompleted ArtNet-Paket from {}", src_addr);
                            }
                        }
                    } else {
                        handle_generic_packet(data, src_addr);
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Timeout erreicht, einfach weitermachen
                }
                Err(e) => {
                    eprintln!("Error receiving UDP-Packets: {}", e);
                    break;
                }
            }
        }
    });

    Ok(())
}

fn is_artnet_packet(data: &[u8]) -> bool {
    data.len() >= 10 && &data[0..8] == ARTNET_HEADER
}

fn handle_generic_packet(data: &[u8], src: std::net::SocketAddr) {
    println!("Received other UDP-Packet {}: {} Bytes", src, data.len());
}
