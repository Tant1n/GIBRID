//! Состояния соединения протокола Minecraft.

/// Состояния соединения Minecraft протокола.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    #[default]
    Handshake = 0,
    Status = 1,
    Login = 2,
    Configuration = 3,
    Play = 4,
}

/// Описание протокола Minecraft 26.3.
#[derive(Debug, Clone)]
pub struct Protocol {
    /// Версия игры
    pub game_version: String,
    /// Версия протокола
    pub protocol_version: i32,
    /// Адрес сервера
    pub server_address: String,
    /// Порт сервера
    pub server_port: u16,
}

impl Protocol {
    pub fn new(game_version: &str, protocol_version: i32, server_address: &str, server_port: u16) -> Self {
        Self {
            game_version: game_version.to_string(),
            protocol_version,
            server_address: server_address.to_string(),
            server_port,
        }
    }
}

impl Default for Protocol {
    fn default() -> Self {
        Self {
            game_version: "26.3".to_string(),
            protocol_version: 777,
            server_address: "127.0.0.1".to_string(),
            server_port: 25565,
        }
    }
}
