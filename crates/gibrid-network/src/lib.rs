//! GIBRID Network - Native Rust Minecraft Java Edition 26.3 Network Protocol
//!
//! Implements the Minecraft Java Edition network protocol:
//! - Handshake (state 0)
//! - Status (state 1) - server ping
//! - Login (state 2) - client authentication
//! - Configuration (state 3) - resource packs, dimensions
//! - Play (state 4) - gameplay packets
//!
//! Status: Phase 6 - Network Protocol Implementation

#![forbid(unsafe_code)]

pub mod codec;
pub mod connection;
pub mod network;
pub mod packet;
pub mod protocol;

pub use codec::{VarInt, VarLong};
pub use connection::Connection;
pub use network::Connection as NetworkConnection;
pub use packet::*;
pub use packet::PacketEncodeDecode;
pub use protocol::{ConnectionState, Protocol};

/// Top-level protocol wrapper alias
pub type GIBRIDProtocol = Protocol;

/// Network errors
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Invalid packet: {0}")]
    InvalidPacket(String),
    #[error("Unsupported packet: {0}")]
    UnsupportedPacket(String),
    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_creation() {
        let protocol = Protocol::new("26.3", 777, "127.0.0.1", 25565);
        assert_eq!(protocol.game_version, "26.3");
        assert_eq!(protocol.protocol_version, 777);
        assert_eq!(protocol.server_address, "127.0.0.1");
        assert_eq!(protocol.server_port, 25565);
    }

    #[test]
    fn protocol_default() {
        let protocol = Protocol::default();
        assert_eq!(protocol.protocol_version, 777);
        assert_eq!(protocol.server_address, "127.0.0.1");
        assert_eq!(protocol.server_port, 25565);
    }

    #[test]
    fn connection_state_default() {
        assert_eq!(ConnectionState::default(), ConnectionState::Handshake);
        assert_eq!(ConnectionState::Status as u8, 1);
        assert_eq!(ConnectionState::Login as u8, 2);
        assert_eq!(ConnectionState::Configuration as u8, 3);
        assert_eq!(ConnectionState::Play as u8, 4);
    }
}