use std::io;
use std::sync::Mutex;
use std::time::Duration;

use crate::interfaces::DmxInterface;
use Common::fixture::MAX_CHANNEL;

// Enttec DMX USB Pro – Nachrichtenformat (API v1.44)
// [0x7E] [Label] [Len_LSB] [Len_MSB] [Data...] [0xE7]
const MSG_START: u8 = 0x7E;
const MSG_END: u8 = 0xE7;

/// Label 6 – "Output Only Send DMX Packet" (Port 1 / Mk1 & Mk2)
const LABEL_DMX_OUTPUT_PORT1: u8 = 6;

/// Label 202 (0xCA) – "Send DMX Packet Port 2" (nur Mk2, zweites Universum)
const LABEL_DMX_OUTPUT_PORT2: u8 = 202;

/// DMX Start-Code (immer 0x00 für normales DMX512)
const DMX_START_CODE: u8 = 0x00;

/// Enttec DMX USB Pro Interface (unterstützt Mk1 und Mk2).
///
/// Der Mk2 hat zwei DMX-Ausgänge:
/// - `local_universe_index = 0` → Port 1 (Label 6)
/// - `local_universe_index = 1` → Port 2 (Label 202, nur Mk2)
///
/// # Beispiel
///
/// ```no_run
/// use crate::interfaces::enttec_dmx_pro::EnttecDmxPro;
///
/// let interface = EnttecDmxPro::new("/dev/ttyUSB0").unwrap();
/// // oder auf Windows: EnttecDmxPro::new("COM3").unwrap();
/// ```
pub struct EnttecDmxPro {
    port: Mutex<Box<dyn serialport::SerialPort>>,
}

impl EnttecDmxPro {
    /// Öffnet die serielle Verbindung zum Enttec DMX USB Pro.
    ///
    /// # Arguments
    ///
    /// * `port_name` - Serieller Port, z.B. `"/dev/ttyUSB0"` (Linux/macOS)
    ///                 oder `"COM3"` (Windows)
    pub fn new(port_name: &str) -> Result<Self, io::Error> {
        let port = serialport::new(port_name, 57600)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(Self { port: Mutex::new(port) })
    }

    /// Listet alle verfügbaren seriellen Ports auf dem System auf.
    /// Nützlich, um den richtigen Port für das Enttec-Gerät zu finden.
    pub fn list_ports() -> Vec<String> {
        serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.port_name)
            .collect()
    }

    /// Baut das Enttec-Nachrichtenpaket zusammen und sendet es.
    ///
    /// Format: [0x7E] [label] [len_lsb] [len_msb] [start_code] [dmx_data...] [0xE7]
    fn send_packet(&self, label: u8, data: &[u8; MAX_CHANNEL as usize]) -> Result<(), io::Error> {
        // Datenlänge = 1 Byte Start-Code + 512 Byte DMX-Daten
        let payload_len: u16 = 1 + MAX_CHANNEL as u16;
        let len_lsb = (payload_len & 0xFF) as u8;
        let len_msb = ((payload_len >> 8) & 0xFF) as u8;

        // Nachricht zusammensetzen
        let mut msg = Vec::with_capacity(5 + payload_len as usize + 1);
        msg.push(MSG_START);
        msg.push(label);
        msg.push(len_lsb);
        msg.push(len_msb);
        msg.push(DMX_START_CODE);
        msg.extend_from_slice(data);
        msg.push(MSG_END);

        // Als einzelnen Write senden – wichtig, damit der FTDI-Treiber das
        // als ein USB-Paket behandelt und kein Framing-Fehler entsteht.
        // Wenn SerialPort intern puffert, erzwingen wir den Flush danach.
        let mut port = self.port.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        port.write_all(&msg)?;
        port.flush()?;

        Ok(())
    }
}

impl DmxInterface for EnttecDmxPro {
    /// Sendet ein DMX-Universum an den Enttec DMX USB Pro.
    ///
    /// # Arguments
    ///
    /// * `local_universe_index` - `0` für Port 1, `1` für Port 2 (nur Mk2)
    /// * `data` - 512 DMX-Kanalwerte
    fn send_universe(
        &self,
        local_universe_index: u16,
        data: &[u8; MAX_CHANNEL as usize],
    ) -> Result<(), io::Error> {
        let label = match local_universe_index {
            0 => LABEL_DMX_OUTPUT_PORT1,
            1 => LABEL_DMX_OUTPUT_PORT2,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "EnttecDmxPro unterstützt maximal 2 Universen (Index 0 oder 1), \
                         aber Index {} wurde angegeben.",
                        local_universe_index
                    ),
                ))
            }
        };

        self.send_packet(label, data)
    }
}