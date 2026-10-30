//! GIBRID Network - Connection implementation
//! Handles handshake, status, login, configuration, and play states

use crate::codec::VarInt;
use crate::packet::*;
use crate::protocol::{ConnectionState, Protocol};
use crate::NetworkError;
use std::io::{Read, Write};
use std::net::TcpStream;

/// Connection to Minecraft server
pub struct Connection {
    stream: TcpStream,
    state: ConnectionState,
    protocol: Protocol,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            state: ConnectionState::Handshake,
            protocol: Protocol::default(),
        }
    }

    /// Current connection state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Perform handshake with server
    pub fn handshake(&mut self, protocol: &Protocol) -> std::io::Result<()> {
        let packet = HandshakePacket::new(
            protocol.protocol_version,
            &protocol.server_address,
            protocol.server_port,
            2, // Login state
        );
        self.send_packet(&Packet::Handshake(packet))?;
        self.state = ConnectionState::Status;
        Ok(())
    }

    /// Send Status Request and receive Status Response
    pub fn status_request(&mut self) -> Result<StatusResponsePacket, NetworkError> {
        if self.state != ConnectionState::Status {
            return Err(NetworkError::Protocol(format!(
                "Expected Status state, got {:?}",
                self.state
            )));
        }

        self.send_packet(&Packet::StatusRequest(StatusRequestPacket))?;
        self.state = ConnectionState::Status;

        let packet_id = VarInt::read(&mut self.stream)?.value();
        let length = VarInt::read(&mut self.stream)?.value() as usize;
        let mut data = vec![0u8; length];
        self.stream.read_exact(&mut data)?;

        let packet = Packet::decode(ConnectionState::Status, packet_id, &data)?;
        match packet {
            Packet::StatusResponse(response) => Ok(response),
            _ => Err(NetworkError::InvalidPacket("Expected StatusResponse".to_string())),
        }
    }

    /// Send Login Start and receive Login Success
    pub fn login_start(&mut self, username: &str) -> Result<LoginSuccessPacket, NetworkError> {
        if self.state != ConnectionState::Status {
            return Err(NetworkError::Protocol(format!(
                "Expected Status state, got {:?}",
                self.state
            )));
        }

        self.send_packet(&Packet::LoginStart(LoginStartPacket::new(username)))?;
        self.state = ConnectionState::Login;

        let packet_id = VarInt::read(&mut self.stream)?.value();
        let length = VarInt::read(&mut self.stream)?.value() as usize;
        let mut data = vec![0u8; length];
        self.stream.read_exact(&mut data)?;

        let packet = Packet::decode(ConnectionState::Login, packet_id, &data)?;
        match packet {
            Packet::LoginSuccess(response) => Ok(response),
            _ => Err(NetworkError::InvalidPacket("Expected LoginSuccess".to_string())),
        }
    }

    /// Send any packet
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

    /// Get raw stream for direct access
    pub fn stream(&mut self) -> &mut TcpStream {
        &mut self.stream
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_state_values() {
        assert_eq!(ConnectionState::Handshake as u8, 0);
        assert_eq!(ConnectionState::Status as u8, 1);
        assert_eq!(ConnectionState::Login as u8, 2);
        assert_eq!(ConnectionState::Configuration as u8, 3);
        assert_eq!(ConnectionState::Play as u8, 4);
    }
}
