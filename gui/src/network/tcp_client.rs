use crate::networking::messages::TcpServerMessage::CommandOutput;
use crate::networking::messages::{
    SubscribeTopic, TcpClientMessage, TcpServerMessage, UpdateMode, UserRole,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

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

    pub fn send_message(&mut self, message: TcpClientMessage) {
        match message {
            TcpClientMessage::Connect { password, user_name, user_role } => {

            }
        }
    }
}
