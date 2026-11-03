// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Оффлайн-аккаунты для локальной игры без Microsoft/Xbox Live.
//! UUID генерируется детерминированно из имени (как в оффлайн-режиме Minecraft).

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Ошибки работы с аккаунтами.
#[derive(Debug, Error)]
pub enum AccountError {
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML: {0}")]
    Toml(#[from] toml::ser::Error),
    #[error("account not found: {0}")]
    NotFound(String),
    #[error("account already exists: {0}")]
    AlreadyExists(String),
    #[error("invalid username: {0}")]
    InvalidUsername(String),
}

/// Оффлайн-аккаунт игрока.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineAccount {
    /// Имя игрока (используется в чате, над головой, в skin).
    pub username: String,
    /// Детерминированный UUID, сгенерированный из имени.
    pub uuid: String,
    /// Время создания аккаунта (Unix timestamp).
    pub created_at: u64,
    /// Последний вход.
    pub last_login: Option<u64>,
}

impl OfflineAccount {
    /// Создать новый оффлайн-аккаунт.
    pub fn new(username: &str) -> Result<Self, AccountError> {
        if username.is_empty() || username.len() > 16 {
            return Err(AccountError::InvalidUsername(username.to_string()));
        }
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(AccountError::InvalidUsername(username.to_string()));
        }
        Ok(Self {
            username: username.to_string(),
            uuid: offline_uuid(username),
            created_at: current_time(),
            last_login: None,
        })
    }

    /// Отметить вход в систему.
    pub fn touch_login(&mut self) {
        self.last_login = Some(current_time());
    }
}

/// Менеджер оффлайн-аккаунтов. Хранит список в TOML-файле.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountManager {
    accounts: Vec<OfflineAccount>,
    current_username: Option<String>,
}

impl Default for AccountManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountManager {
    /// Создать пустой менеджер.
    pub fn new() -> Self {
        Self {
            accounts: Vec::new(),
            current_username: None,
        }
    }

    /// Добавить аккаунт. Если с таким именем уже есть — ошибка.
    pub fn add_account(&mut self, username: &str) -> Result<&OfflineAccount, AccountError> {
        if self.accounts.iter().any(|a| a.username == username) {
            return Err(AccountError::AlreadyExists(username.to_string()));
        }
        let account = OfflineAccount::new(username)?;
        self.accounts.push(account.clone());
        Ok(self.accounts.last_mut().unwrap())
    }

    /// Получить аккаунт по имени.
    pub fn get_account(&self, username: &str) -> Option<&OfflineAccount> {
        self.accounts.iter().find(|a| a.username == username)
    }

    /// Получить mutable ссылку на аккаунт.
    pub fn get_account_mut(&mut self, username: &str) -> Option<&mut OfflineAccount> {
        self.accounts.iter_mut().find(|a| a.username == username)
    }

    /// Установить текущий аккаунт.
    pub fn set_current(&mut self, username: &str) -> Result<(), AccountError> {
        if self.get_account(username).is_none() {
            return Err(AccountError::NotFound(username.to_string()));
        }
        self.current_username = Some(username.to_string());
        Ok(())
    }

    /// Получить текущий аккаунт.
    pub fn current_account(&self) -> Option<&OfflineAccount> {
        self.current_username
            .as_deref()
            .and_then(|u| self.get_account(u))
    }

    /// Текущее имя игрока.
    pub fn current_username(&self) -> Option<&str> {
        self.current_username.as_deref()
    }

/// Список всех аккаунтов.
    pub fn accounts(&self) -> &[OfflineAccount] {
        &self.accounts
    }

    /// Удалить аккаунт по имени.
    pub fn remove_account(&mut self, username: &str) -> Result<(), AccountError> {
        let idx = self
            .accounts
            .iter()
            .position(|a| a.username == username)
            .ok_or_else(|| AccountError::NotFound(username.to_string()))?;
        self.accounts.remove(idx);
        if self.current_username.as_deref() == Some(username) {
            self.current_username = self.accounts.first().map(|a| a.username.clone());
        }
        Ok(())
    }

    /// Загрузить менеджер из TOML-файла.
    pub fn load(path: &Path) -> Result<Self, AccountError> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let text = std::fs::read_to_string(path).map_err(AccountError::Io)?;
        let data: AccountManager = toml::from_str(&text).map_err(|e| {
            AccountError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
        Ok(data)
    }

    /// Сохранить менеджер в TOML-файл.
    pub fn save(&self, path: &Path) -> Result<(), AccountError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(AccountError::Io)?;
        }
        let text = toml::to_string_pretty(self).map_err(AccountError::Toml)?;
        std::fs::write(path, text).map_err(AccountError::Io)?;
        Ok(())
    }
}

/// Детерминированный UUID из имени (оффлайн-алгоритм Minecraft).
/// Формат: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx, где байты берутся из MD5-like хеша имени.
fn offline_uuid(username: &str) -> String {
    // Простой детерминированный 128-битный хеш (FNV-1a 64-bit x2).
    let mut h1: u64 = 14695981039346656037;
    let mut h2: u64 = 14695981039346656037;
    for byte in username.bytes() {
        h1 ^= byte as u64;
        h1 = h1.wrapping_mul(1099511628211);
        h2 ^= byte as u64;
        h2 = h2.wrapping_mul(1099511628211);
    }
    let bytes = [
        (h1 >> 56) as u8, (h1 >> 48) as u8, (h1 >> 40) as u8, (h1 >> 32) as u8,
        (h1 >> 24) as u8, (h1 >> 16) as u8, (h1 >> 8) as u8, h1 as u8,
        (h2 >> 56) as u8, (h2 >> 48) as u8, (h2 >> 40) as u8, (h2 >> 32) as u8,
        (h2 >> 24) as u8, (h2 >> 16) as u8, (h2 >> 8) as u8, h2 as u8,
    ];
    // Версия 4 (случайный) — устанавливаем биты версии и варианта.
    let mut b = bytes;
    b[6] = (b[6] & 0x0f) | 0x40; // версия 4
    b[8] = (b[8] & 0x3f) | 0x80; // вариант RFC 4122
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

fn current_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_get_account() {
        let mut mgr = AccountManager::new();
        mgr.add_account("Steve").unwrap();
        let acc = mgr.get_account("Steve").unwrap();
        assert_eq!(acc.username, "Steve");
        assert!(!acc.uuid.is_empty());
    }

    #[test]
    fn duplicate_account_fails() {
        let mut mgr = AccountManager::new();
        mgr.add_account("Steve").unwrap();
        assert!(mgr.add_account("Steve").is_err());
    }

    #[test]
    fn set_current_account() {
        let mut mgr = AccountManager::new();
        mgr.add_account("Steve").unwrap();
        mgr.add_account("Alex").unwrap();
        mgr.set_current("Alex").unwrap();
        assert_eq!(mgr.current_username(), Some("Alex"));
    }

    #[test]
    fn offline_uuid_is_deterministic() {
        let u1 = offline_uuid("Steve");
        let u2 = offline_uuid("Steve");
        assert_eq!(u1, u2);
        assert_ne!(offline_uuid("Steve"), offline_uuid("Alex"));
    }

    #[test]
    fn save_and_load() {
        let mut mgr = AccountManager::new();
        mgr.add_account("TestUser").unwrap();
        mgr.set_current("TestUser").unwrap();
        let path = std::env::temp_dir().join("gibrid_test_accounts.toml");
        let _ = mgr.save(&path);
        let loaded = AccountManager::load(&path).unwrap();
        assert_eq!(loaded.current_username(), Some("TestUser"));
        let _ = std::fs::remove_file(&path);
    }
}
