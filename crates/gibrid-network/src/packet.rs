// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.//! GIBRID Network - Protocol Packet Definitions
//! Minecraft Java Edition 26.3 protocol packets

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crate::codec::{VarInt, VarIntString};
use crate::NetworkError;
use std::io::{Cursor, Read};

/// All packet types supported by GIBRID
#[derive(Debug, Clone)]
pub enum Packet {
    Handshake(HandshakePacket),
    StatusRequest(StatusRequestPacket),
    StatusResponse(StatusResponsePacket),
    LoginStart(LoginStartPacket),
    LoginSuccess(LoginSuccessPacket),
    PlayJoinGame(PlayJoinGamePacket),
    PlaySpawnPosition(PlaySpawnPositionPacket),
    PlayBlockChange(BlockChangePacket),
    PlayChatMessage(ChatMessagePacket),
    PlaySetExperience(SetExperiencePacket),
    PlayUpdateHealth(UpdateHealthPacket),
    PlayRespawn(RespawnPacket),
    PlayDisconnect(DisconnectPacket),
    PlayKeepAlive(KeepAlivePacket),
    PlayChunkData(ChunkDataPacket),
    PlayBlockChangeAck(BlockChangeAckPacket),
    PlayerPosition(PlayerPositionPacket),
    PlayerPositionLook(PlayerPositionLookPacket),
    PlayerLook(PlayerLookPacket),
    PlayerMovement(PlayerMovementPacket),
}

/// Packet encoding/decoding trait
pub trait PacketEncodeDecode {
    fn encode_payload(&self) -> Vec<u8>;
    fn packet_id(state: crate::protocol::ConnectionState) -> VarInt;
    fn decode(state: crate::protocol::ConnectionState, id: i32, data: &[u8]) -> Result<Packet, NetworkError>;
}

impl PacketEncodeDecode for Packet {
    fn encode_payload(&self) -> Vec<u8> {
        match self {
            Packet::Handshake(p) => p.encode(),
            Packet::StatusRequest(p) => p.encode(),
            Packet::StatusResponse(p) => p.encode(),
            Packet::LoginStart(p) => p.encode(),
            Packet::LoginSuccess(p) => p.encode(),
            Packet::PlayJoinGame(p) => p.encode(),
            Packet::PlaySpawnPosition(p) => p.encode(),
            Packet::PlayBlockChange(p) => p.encode(),
            Packet::PlayChatMessage(p) => p.encode(),
            Packet::PlaySetExperience(p) => p.encode(),
            Packet::PlayUpdateHealth(p) => p.encode(),
            Packet::PlayRespawn(p) => p.encode(),
            Packet::PlayDisconnect(p) => p.encode(),
            Packet::PlayKeepAlive(p) => p.encode(),
            Packet::PlayChunkData(p) => p.encode(),
            Packet::PlayBlockChangeAck(p) => p.encode(),
            Packet::PlayerPosition(p) => p.encode(),
            Packet::PlayerPositionLook(p) => p.encode(),
            Packet::PlayerLook(p) => p.encode(),
            Packet::PlayerMovement(p) => p.encode(),
        }
    }

    fn packet_id(state: crate::protocol::ConnectionState) -> VarInt {
        // Default packet ID based on state - serverbound packets start at 0
        match state {
            crate::protocol::ConnectionState::Handshake => VarInt::new(0),
            crate::protocol::ConnectionState::Status => VarInt::new(0),
            crate::protocol::ConnectionState::Login => VarInt::new(0),
            crate::protocol::ConnectionState::Configuration => VarInt::new(0),
            crate::protocol::ConnectionState::Play => VarInt::new(0),
        }
    }

    fn decode(state: crate::protocol::ConnectionState, id: i32, data: &[u8]) -> Result<Packet, NetworkError> {
        match state {
            crate::protocol::ConnectionState::Handshake => {
                match id {
                    0 => Ok(Packet::Handshake(HandshakePacket::decode(data)?)),
                    _ => Err(NetworkError::UnsupportedPacket(format!("Handshake packet id {}", id))),
                }
            }
            crate::protocol::ConnectionState::Status => {
                match id {
                    0 => Ok(Packet::StatusRequest(StatusRequestPacket::decode(data)?)),
                    1 => Ok(Packet::StatusResponse(StatusResponsePacket::decode(data)?)),
                    _ => Err(NetworkError::UnsupportedPacket(format!("Status packet id {}", id))),
                }
            }
            crate::protocol::ConnectionState::Login => {
                match id {
                    0 => Ok(Packet::LoginStart(LoginStartPacket::decode(data)?)),
                    1 => Ok(Packet::LoginSuccess(LoginSuccessPacket::decode(data)?)),
                    _ => Err(NetworkError::UnsupportedPacket(format!("Login packet id {}", id))),
                }
            }
            crate::protocol::ConnectionState::Configuration => {
                Err(NetworkError::UnsupportedPacket("Configuration state not yet implemented".to_string()))
            }
            crate::protocol::ConnectionState::Play => {
                match id {
                    0 => Ok(Packet::PlayJoinGame(PlayJoinGamePacket::decode(data)?)),
                    1 => Ok(Packet::PlaySpawnPosition(PlaySpawnPositionPacket::decode(data)?)),
                    2 => Ok(Packet::PlayBlockChange(BlockChangePacket::decode(data)?)),
                    3 => Ok(Packet::PlayChatMessage(ChatMessagePacket::decode(data)?)),
                    4 => Ok(Packet::PlaySetExperience(SetExperiencePacket::decode(data)?)),
                    5 => Ok(Packet::PlayUpdateHealth(UpdateHealthPacket::decode(data)?)),
                    6 => Ok(Packet::PlayRespawn(RespawnPacket::decode(data)?)),
                    7 => Ok(Packet::PlayDisconnect(DisconnectPacket::decode(data)?)),
                    8 => Ok(Packet::PlayKeepAlive(KeepAlivePacket::decode(data)?)),
                    9 => Ok(Packet::PlayChunkData(ChunkDataPacket::decode(data)?)),
                    10 => Ok(Packet::PlayBlockChangeAck(BlockChangeAckPacket::decode(data)?)),
                    _ => Err(NetworkError::UnsupportedPacket(format!("Play packet id {}", id))),
                }
            }
        }
    }
}

// ===== Handshake =====

#[derive(Debug, Clone)]
pub struct HandshakePacket {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,
}

impl HandshakePacket {
    pub fn new(protocol_version: i32, server_address: &str, server_port: u16, next_state: i32) -> Self {
        Self {
            protocol_version,
            server_address: server_address.to_string(),
            server_port,
            next_state,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        VarInt::new(self.protocol_version).write(&mut buf).unwrap();
        VarIntString::new(self.server_address.clone()).write(&mut buf).unwrap();
        buf.write_u16::<BigEndian>(self.server_port).unwrap();
        VarInt::new(self.next_state).write(&mut buf).unwrap();
        buf
    }

    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        let protocol_version = VarInt::read(&mut cursor)?.value();
        let server_address = VarIntString::read(&mut cursor)?.value().to_string();
        let server_port = cursor.read_u16::<BigEndian>()?;
        let next_state = VarInt::read(&mut cursor)?.value();
        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}

// ===== Status =====

#[derive(Debug, Clone)]
pub struct StatusRequestPacket;

impl StatusRequestPacket {
    pub fn new() -> Self { Self }
    pub fn encode(&self) -> Vec<u8> { Vec::new() }
    pub fn decode(_data: &[u8]) -> Result<Self, NetworkError> { Ok(Self) }
}

#[derive(Debug, Clone)]
pub struct StatusResponsePacket {
    pub json_response: String,
}

impl StatusResponsePacket {
    pub fn new(json: &str) -> Self { Self { json_response: json.to_string() } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        VarIntString::new(self.json_response.clone()).write(&mut buf).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let json = VarIntString::read(&mut Cursor::new(data))?.value().to_string();
        Ok(Self { json_response: json })
    }
}

// ===== Login =====

#[derive(Debug, Clone)]
pub struct LoginStartPacket {
    pub username: String,
}

impl LoginStartPacket {
    pub fn new(username: &str) -> Self { Self { username: username.to_string() } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        VarIntString::new(self.username.clone()).write(&mut buf).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let username = VarIntString::read(&mut Cursor::new(data))?.value().to_string();
        Ok(Self { username })
    }
}

#[derive(Debug, Clone)]
pub struct LoginSuccessPacket {
    pub uuid: String,
    pub username: String,
}

impl LoginSuccessPacket {
    pub fn new(uuid: &str, username: &str) -> Self { Self { uuid: uuid.to_string(), username: username.to_string() } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        VarIntString::new(self.uuid.clone()).write(&mut buf).unwrap();
        VarIntString::new(self.username.clone()).write(&mut buf).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        let uuid = VarIntString::read(&mut cursor)?.value().to_string();
        let username = VarIntString::read(&mut cursor)?.value().to_string();
        Ok(Self { uuid, username })
    }
}

// ===== Play =====

#[derive(Debug, Clone)]
pub struct PlayJoinGamePacket {
    pub entity_id: i32,
    pub gamemode: i32,
    pub dimension: String,
}

impl PlayJoinGamePacket {
    pub fn new(entity_id: i32, gamemode: i32, dimension: &str) -> Self {
        Self { entity_id, gamemode, dimension: dimension.to_string() }
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_i32::<BigEndian>(self.entity_id).unwrap();
        buf.write_i32::<BigEndian>(self.gamemode).unwrap();
        VarIntString::new(self.dimension.clone()).write(&mut buf).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        let entity_id = cursor.read_i32::<BigEndian>()?;
        let gamemode = cursor.read_i32::<BigEndian>()?;
        let dimension = VarIntString::read(&mut cursor)?.value().to_string();
        Ok(Self { entity_id, gamemode, dimension })
    }
}

#[derive(Debug, Clone)]
pub struct PlaySpawnPositionPacket {
    pub x: i32, pub y: i32, pub z: i32,
}

impl PlaySpawnPositionPacket {
    pub fn new(x: i32, y: i32, z: i32) -> Self { Self { x, y, z } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_i32::<BigEndian>(self.x).unwrap();
        buf.write_i32::<BigEndian>(self.y).unwrap();
        buf.write_i32::<BigEndian>(self.z).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        Ok(Self {
            x: cursor.read_i32::<BigEndian>()?,
            y: cursor.read_i32::<BigEndian>()?,
            z: cursor.read_i32::<BigEndian>()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BlockChangePacket {
    pub x: i32, pub y: i32, pub z: i32, pub block_id: String,
}

impl BlockChangePacket {
    pub fn new(x: i32, y: i32, z: i32, block_id: &str) -> Self {
        Self { x, y, z, block_id: block_id.to_string() }
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_i32::<BigEndian>(self.x).unwrap();
        buf.write_i32::<BigEndian>(self.y).unwrap();
        buf.write_i32::<BigEndian>(self.z).unwrap();
        VarIntString::new(self.block_id.clone()).write(&mut buf).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        Ok(Self {
            x: cursor.read_i32::<BigEndian>()?,
            y: cursor.read_i32::<BigEndian>()?,
            z: cursor.read_i32::<BigEndian>()?,
            block_id: VarIntString::read(&mut cursor)?.value().to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessagePacket {
    pub message: String,
}

impl ChatMessagePacket {
    pub fn new(message: &str) -> Self { Self { message: message.to_string() } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        VarIntString::new(self.message.clone()).write(&mut buf).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let message = VarIntString::read(&mut Cursor::new(data))?.value().to_string();
        Ok(Self { message })
    }
}

#[derive(Debug, Clone)]
pub struct SetExperiencePacket {
    pub experience: f32,
    pub level: i32,
    pub total_experience: i32,
}

impl SetExperiencePacket {
    pub fn new(experience: f32, level: i32, total: i32) -> Self { Self { experience, level, total_experience: total } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_f32::<BigEndian>(self.experience).unwrap();
        buf.write_i32::<BigEndian>(self.level).unwrap();
        buf.write_i32::<BigEndian>(self.total_experience).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        Ok(Self {
            experience: cursor.read_f32::<BigEndian>()?,
            level: cursor.read_i32::<BigEndian>()?,
            total_experience: cursor.read_i32::<BigEndian>()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct UpdateHealthPacket {
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
}

impl UpdateHealthPacket {
    pub fn new(health: f32, food: i32, saturation: f32) -> Self { Self { health, food, saturation } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_f32::<BigEndian>(self.health).unwrap();
        buf.write_i32::<BigEndian>(self.food).unwrap();
        buf.write_f32::<BigEndian>(self.saturation).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        Ok(Self {
            health: cursor.read_f32::<BigEndian>()?,
            food: cursor.read_i32::<BigEndian>()?,
            saturation: cursor.read_f32::<BigEndian>()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RespawnPacket {
    pub dimension: String,
}

impl RespawnPacket {
    pub fn new(dimension: &str) -> Self { Self { dimension: dimension.to_string() } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        VarIntString::new(self.dimension.clone()).write(&mut buf).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let dimension = VarIntString::read(&mut Cursor::new(data))?.value().to_string();
        Ok(Self { dimension })
    }
}

#[derive(Debug, Clone)]
pub struct DisconnectPacket {
    pub reason: String,
}

impl DisconnectPacket {
    pub fn new(reason: &str) -> Self { Self { reason: reason.to_string() } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        VarIntString::new(self.reason.clone()).write(&mut buf).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let reason = VarIntString::read(&mut Cursor::new(data))?.value().to_string();
        Ok(Self { reason })
    }
}

#[derive(Debug, Clone)]
pub struct KeepAlivePacket {
    pub id: i32,
}

impl KeepAlivePacket {
    pub fn new(id: i32) -> Self { Self { id } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_i32::<BigEndian>(self.id).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        Ok(Self { id: cursor.read_i32::<BigEndian>()? })
    }
}

#[derive(Debug, Clone)]
pub struct ChunkDataPacket {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub data: Vec<u8>,
}

impl ChunkDataPacket {
    pub fn new(x: i32, z: i32, data: Vec<u8>) -> Self { Self { chunk_x: x, chunk_z: z, data } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_i32::<BigEndian>(self.chunk_x).unwrap();
        buf.write_i32::<BigEndian>(self.chunk_z).unwrap();
        buf.write_i32::<BigEndian>(self.data.len() as i32).unwrap();
        buf.extend_from_slice(&self.data);
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        let cx = cursor.read_i32::<BigEndian>()?;
        let cz = cursor.read_i32::<BigEndian>()?;
        let len = cursor.read_i32::<BigEndian>()? as usize;
        let mut data = vec![0u8; len];
        cursor.read_exact(&mut data)?;
        Ok(Self { chunk_x: cx, chunk_z: cz, data })
    }
}

#[derive(Debug, Clone)]
pub struct BlockChangeAckPacket {
    pub sequence_id: i32,
}

impl BlockChangeAckPacket {
    pub fn new(sequence_id: i32) -> Self { Self { sequence_id } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_i32::<BigEndian>(self.sequence_id).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        Ok(Self { sequence_id: cursor.read_i32::<BigEndian>()? })
    }
}

#[derive(Debug, Clone)]
pub struct PlayerPositionPacket {
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: f32, pub pitch: f32,
    pub on_ground: bool,
}

impl PlayerPositionPacket {
    pub fn new(x: f64, y: f64, z: f64, yaw: f32, pitch: f32) -> Self {
        Self { x, y, z, yaw, pitch, on_ground: false }
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_f64::<BigEndian>(self.x).unwrap();
        buf.write_f64::<BigEndian>(self.y).unwrap();
        buf.write_f64::<BigEndian>(self.z).unwrap();
        buf.write_f32::<BigEndian>(self.yaw).unwrap();
        buf.write_f32::<BigEndian>(self.pitch).unwrap();
        buf.write_u8(if self.on_ground { 1 } else { 0 }).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        Ok(Self {
            x: cursor.read_f64::<BigEndian>()?,
            y: cursor.read_f64::<BigEndian>()?,
            z: cursor.read_f64::<BigEndian>()?,
            yaw: cursor.read_f32::<BigEndian>()?,
            pitch: cursor.read_f32::<BigEndian>()?,
            on_ground: cursor.read_u8()? != 0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PlayerPositionLookPacket {
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: f32, pub pitch: f32,
    pub on_ground: bool,
}

impl PlayerPositionLookPacket {
    pub fn new(x: f64, y: f64, z: f64, yaw: f32, pitch: f32, on_ground: bool) -> Self {
        Self { x, y, z, yaw, pitch, on_ground }
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_f64::<BigEndian>(self.x).unwrap();
        buf.write_f64::<BigEndian>(self.y).unwrap();
        buf.write_f64::<BigEndian>(self.z).unwrap();
        buf.write_f32::<BigEndian>(self.yaw).unwrap();
        buf.write_f32::<BigEndian>(self.pitch).unwrap();
        buf.write_u8(if self.on_ground { 1 } else { 0 }).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        Ok(Self {
            x: cursor.read_f64::<BigEndian>()?,
            y: cursor.read_f64::<BigEndian>()?,
            z: cursor.read_f64::<BigEndian>()?,
            yaw: cursor.read_f32::<BigEndian>()?,
            pitch: cursor.read_f32::<BigEndian>()?,
            on_ground: cursor.read_u8()? != 0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PlayerLookPacket {
    pub yaw: f32, pub pitch: f32,
    pub on_ground: bool,
}

impl PlayerLookPacket {
    pub fn new(yaw: f32, pitch: f32, on_ground: bool) -> Self { Self { yaw, pitch, on_ground } }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_f32::<BigEndian>(self.yaw).unwrap();
        buf.write_f32::<BigEndian>(self.pitch).unwrap();
        buf.write_u8(if self.on_ground { 1 } else { 0 }).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        Ok(Self {
            yaw: cursor.read_f32::<BigEndian>()?,
            pitch: cursor.read_f32::<BigEndian>()?,
            on_ground: cursor.read_u8()? != 0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PlayerMovementPacket {
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: f32, pub pitch: f32,
    pub on_ground: bool,
}

impl PlayerMovementPacket {
    pub fn new(x: f64, y: f64, z: f64, yaw: f32, pitch: f32, on_ground: bool) -> Self {
        Self { x, y, z, yaw, pitch, on_ground }
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_f64::<BigEndian>(self.x).unwrap();
        buf.write_f64::<BigEndian>(self.y).unwrap();
        buf.write_f64::<BigEndian>(self.z).unwrap();
        buf.write_f32::<BigEndian>(self.yaw).unwrap();
        buf.write_f32::<BigEndian>(self.pitch).unwrap();
        buf.write_u8(if self.on_ground { 1 } else { 0 }).unwrap();
        buf
    }
    pub fn decode(data: &[u8]) -> Result<Self, NetworkError> {
        let mut cursor = Cursor::new(data);
        Ok(Self {
            x: cursor.read_f64::<BigEndian>()?,
            y: cursor.read_f64::<BigEndian>()?,
            z: cursor.read_f64::<BigEndian>()?,
            yaw: cursor.read_f32::<BigEndian>()?,
            pitch: cursor.read_f32::<BigEndian>()?,
            on_ground: cursor.read_u8()? != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_roundtrip() {
        let p = HandshakePacket::new(777, "127.0.0.1", 25565, 2);
        let encoded = p.encode();
        let decoded = HandshakePacket::decode(&encoded).unwrap();
        assert_eq!(decoded.protocol_version, 777);
        assert_eq!(decoded.server_address, "127.0.0.1");
        assert_eq!(decoded.server_port, 25565);
        assert_eq!(decoded.next_state, 2);
    }

    #[test]
    fn varint_roundtrip() {
        let values = vec![0, 1, 127, 128, 255, 1000, -1];
        for v in values {
            let vi = VarInt::new(v);
            let mut buf = Vec::new();
            vi.write(&mut buf).unwrap();
            let mut cursor = Cursor::new(&buf);
            let d = VarInt::read(&mut cursor).unwrap();
            assert_eq!(vi.value(), d.value());
        }
    }

    #[test]
    fn status_response_roundtrip() {
        let p = StatusResponsePacket::new("{\"version\":{\"name\":\"26.3\",\"protocol\":777}}");
        let encoded = p.encode();
        let decoded = StatusResponsePacket::decode(&encoded).unwrap();
        assert_eq!(decoded.json_response, p.json_response);
    }

    #[test]
    fn player_position_roundtrip() {
        let p = PlayerPositionPacket::new(100.5, 64.0, 200.5, 0.0, 0.0);
        let encoded = p.encode();
        let decoded = PlayerPositionPacket::decode(&encoded).unwrap();
        assert!((decoded.x - p.x).abs() < f64::EPSILON);
        assert!((decoded.y - p.y).abs() < f64::EPSILON);
        assert!((decoded.z - p.z).abs() < f64::EPSILON);
        assert_eq!(decoded.yaw, p.yaw);
        assert_eq!(decoded.pitch, p.pitch);
        assert_eq!(decoded.on_ground, p.on_ground);
    }
}
