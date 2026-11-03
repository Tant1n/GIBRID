// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.//! GIBRID Network - Connection implementation
//! Handles handshake, status, login, configuration, and play states

use crate::codec::VarInt;
use crate::packet::*;
use crate::protocol::{ConnectionState, Protocol};
use crate::NetworkError;
use std::io::{Read, Write};
use std::net::TcpStream;

/// Network connection wrapper
pub struct Connection {
    stream: TcpStream,
    state: ConnectionState,
    protocol: Protocol,
    compression_enabled: bool,
    compression_threshold: i32,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            state: ConnectionState::Handshake,
            protocol: Protocol::default(),
            compression_enabled: false,
            compression_threshold: 256,
        }
    }

    /// Send a handshake packet to initiate connection
    pub fn handshake(&mut self, protocol: &Protocol) -> std::io::Result<()> {
        let packet = Packet::Handshake(HandshakePacket {
            protocol_version: protocol.protocol_version,
            server_address: protocol.server_address.clone(),
            server_port: protocol.server_port,
            next_state: match self.state {
                ConnectionState::Status => 1,
                ConnectionState::Login => 2,
                _ => 1,
            },
        });
        self.send_packet(&packet)?;
        self.state = ConnectionState::Handshake;
        Ok(())
    }

    /// Send status request and get response
    pub fn status_request(&mut self) -> Result<StatusResponsePacket, NetworkError> {
        if self.state != ConnectionState::Handshake {
            return Err(NetworkError::Protocol("Must handshake first".to_string()));
        }

        self.send_packet(&Packet::StatusRequest(StatusRequestPacket))?;
        self.state = ConnectionState::Status;

        // Read response packet
        let packet_id = VarInt::read(&mut self.stream)?.value();
        let length = VarInt::read(&mut self.stream)?.value() as usize;
        let mut data = vec![0u8; length];
        self.stream.read_exact(&mut data)?;

        let packet = Packet::decode(ConnectionState::Status, packet_id, &data)?;
        match packet {
            Packet::StatusResponse(p) => Ok(p),
            _ => Err(NetworkError::InvalidPacket("Expected StatusResponse".to_string())),
        }
    }

    /// Send login start
    pub fn login_start(&mut self, username: &str) -> Result<LoginSuccessPacket, NetworkError> {
        self.send_packet(&Packet::LoginStart(LoginStartPacket::new(username)))?;
        self.state = ConnectionState::Login;

        // Read login response
        let packet_id = VarInt::read(&mut self.stream)?.value();
        let length = VarInt::read(&mut self.stream)?.value() as usize;
        let mut data = vec![0u8; length];
        self.stream.read_exact(&mut data)?;

        let packet = Packet::decode(ConnectionState::Login, packet_id, &data)?;
        match packet {
            Packet::LoginSuccess(p) => Ok(p),
            _ => Err(NetworkError::InvalidPacket("Expected LoginSuccess".to_string())),
        }
    }

    /// Send a generic packet
    pub fn send_packet(&mut self, packet: &Packet) -> std::io::Result<()> {
        let payload = packet.encode_payload();
        let packet_id = Packet::packet_id(self.state);

        let mut buf = Vec::new();
        VarInt::new(packet_id.value() as i32).write(&mut buf)?;
        buf.extend_from_slice(&payload);

        VarInt::new(buf.len() as i32).write(&mut self.stream)?;
        self.stream.write_all(&buf)?;
        self.stream.flush()?;
        Ok(())
    }

    /// Read a packet from the stream
    pub fn read_packet(&mut self) -> Result<Packet, NetworkError> {
        let packet_id = VarInt::read(&mut self.stream)?.value();
        let length = VarInt::read(&mut self.stream)?.value() as usize;
        let mut data = vec![0u8; length];
        self.stream.read_exact(&mut data)?;

        Packet::decode(self.state, packet_id, &data)
    }

    /// Get current connection state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Set connection state
    pub fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_state_transition() {
        // Just verify state values are correct
        assert_eq!(ConnectionState::Handshake as u8, 0);
        assert_eq!(ConnectionState::Status as u8, 1);
        assert_eq!(ConnectionState::Login as u8, 2);
        assert_eq!(ConnectionState::Configuration as u8, 3);
        assert_eq!(ConnectionState::Play as u8, 4);
    }
}

