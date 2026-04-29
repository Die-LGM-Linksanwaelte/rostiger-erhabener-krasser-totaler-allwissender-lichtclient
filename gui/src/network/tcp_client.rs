 use crate::network::tcp_message::TcpMessage;

pub struct TcpClient {
    ip: String,
    socket: std::net::TcpStream,
    is_connected: bool,
}

impl TcpClient {
    pub fn new() -> Self {
        Self {
            socket: std::net::TcpStream::connect().unwrap(),
        }
    }

    pub fn connect(&mut self) {
        self.socket = std::net::TcpStream::connect().unwrap();
    }

    pub fn send_message(&mut self, message: TcpMessage) {
        match message {
            TcpMessage::createFixture() => {
                self.socket
                    .write_all(TcpMessage::createFixture().as_bytes())
                    .unwrap();
            }
        }
    }
}
