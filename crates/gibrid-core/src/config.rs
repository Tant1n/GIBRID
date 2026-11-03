// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.
use serde::Deserialize;
use thiserror::Error;

const CURRENT_SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AppConfig {
    pub schema_version: u32,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA,
            log_level: default_log_level(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("failed to parse config: {0}")]
    Parse(String),
    #[error("unsupported config schema_version {found}; expected {CURRENT_SCHEMA}")]
    Schema { found: u32 },
    #[error("log_level `{0}` is not one of error, warn, info, debug, trace")]
    LogLevel(String),
}

impl AppConfig {
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let cfg: Self = toml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema_version != CURRENT_SCHEMA {
            return Err(ConfigError::Schema {
                found: self.schema_version,
            });
        }
        match self.log_level.as_str() {
            "error" | "warn" | "info" | "debug" | "trace" => Ok(()),
            other => Err(ConfigError::LogLevel(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        AppConfig::default().validate().unwrap();
    }

    #[test]
    fn rejects_unknown_schema() {
        let err = AppConfig::parse("schema_version = 99\n").unwrap_err();
        assert!(matches!(err, ConfigError::Schema { found: 99 }));
    }

    #[test]
    fn rejects_bad_log_level() {
        let err = AppConfig::parse("schema_version = 1\nlog_level = \"verbose\"\n").unwrap_err();
        assert!(matches!(err, ConfigError::LogLevel(_)));
    }
}

