//! GIBRID Java Mod Bridge — опциональная подсистема совместимости с Java-модами.
//!
//! Статус: SPECIFIED (архитектура описана, реализация отложена на Phase 7/8).
//!
//! План:
//! - L0: Данные и ассеты (без кода) — перенос блоков, предметов, рецептов
//! - L1: Портирование кода (Rust → WASM)
//! - L2: JVM-мост (исследование) — загрузка Fabric/NeoForge модов через JNI/FFI

use thiserror::Error;

/// Ошибки Java Mod Bridge.
#[derive(Debug, Error)]
pub enum JavaBridgeError {
    #[error("JVM not found")]
    JvmNotFound,
    #[error("JNI error: {0}")]
    JniError(String),
    #[error("Class not found: {0}")]
    ClassNotFound(String),
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    #[error("Unsupported mod version: {0}")]
    UnsupportedModVersion(String),
    #[error("Bridge not initialized")]
    NotInitialized,
}

/// Уровень поддержки Java-модов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportLevel {
    /// Данные и ассеты только
    L0,
    /// Портирование кода на Rust/WASM
    L1,
    /// JVM-мост (исследование)
    L2,
}

/// Информация о поддерживаемом моде.
#[derive(Debug, Clone)]
pub struct ModInfo {
    pub id: String,
    pub version: String,
    pub support_level: SupportLevel,
    pub license: Option<String>,
}

/// Статус Java Mod Bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeStatus {
    NotInitialized,
    Initialized,
    Failed(String),
}

/// Java Mod Bridge — опциональная подсистема.
pub struct JavaBridge {
    status: BridgeStatus,
    supported_mods: Vec<ModInfo>,
}

impl JavaBridge {
    pub fn new() -> Self {
        Self {
            status: BridgeStatus::NotInitialized,
            supported_mods: Vec::new(),
        }
    }

    pub fn status(&self) -> BridgeStatus {
        self.status.clone()
    }

    pub fn supported_mods(&self) -> &[ModInfo] {
        &self.supported_mods
    }

    pub fn is_mod_supported(&self, mod_id: &str) -> bool {
        self.supported_mods.iter().any(|m| m.id == mod_id)
    }

    /// Инициализировать мост (заглушка — фактическая реализация в Phase 7+).
    pub fn init(&mut self) -> Result<(), JavaBridgeError> {
        self.status = BridgeStatus::Initialized;
        Ok(())
    }

    /// Добавить поддерживаемый мод.
    pub fn add_mod(&mut self, mod_info: ModInfo) {
        self.supported_mods.push(mod_info);
    }
}

impl Default for JavaBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_default() {
        let bridge = JavaBridge::new();
        assert_eq!(bridge.status(), BridgeStatus::NotInitialized);
        assert!(bridge.supported_mods().is_empty());
    }

    #[test]
    fn bridge_init() {
        let mut bridge = JavaBridge::new();
        assert!(bridge.init().is_ok());
        assert_eq!(bridge.status(), BridgeStatus::Initialized);
    }

    #[test]
    fn bridge_add_mod() {
        let mut bridge = JavaBridge::new();
        bridge.add_mod(ModInfo {
            id: "test_mod".to_string(),
            version: "1.0.0".to_string(),
            support_level: SupportLevel::L0,
            license: Some("MIT".to_string()),
        });
        assert!(bridge.is_mod_supported("test_mod"));
        assert!(!bridge.is_mod_supported("unknown_mod"));
    }
}
