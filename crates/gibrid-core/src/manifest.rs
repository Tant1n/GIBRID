use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Machine-readable target version. Code must not invent protocol or data versions.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VersionManifest {
    pub game_version: String,
    pub protocol_version: i32,
    #[serde(default)]
    pub protocol_verified: bool,
    pub data_version: Option<i32>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub verified_on: String,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub dependent_modules: Vec<String>,
    #[serde(default)]
    pub fixture_ids: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("failed to read version manifest {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse version manifest: {0}")]
    Parse(String),
    #[error("game_version must not be empty")]
    EmptyGameVersion,
    #[error("version manifest not found from {start}")]
    NotFound { start: String },
}

impl VersionManifest {
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let manifest: Self =
            toml::from_str(text).map_err(|e| ManifestError::Parse(e.to_string()))?;
        if manifest.game_version.trim().is_empty() {
            return Err(ManifestError::EmptyGameVersion);
        }
        Ok(manifest)
    }

    pub fn load_path(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| ManifestError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Search `start` and parents for `gibrid-version.toml`.
    pub fn load_from_dir(start: impl AsRef<Path>) -> Result<(PathBuf, Self), ManifestError> {
        let start = start.as_ref();
        let mut dir = start.to_path_buf();
        loop {
            let candidate = dir.join("gibrid-version.toml");
            if candidate.is_file() {
                let manifest = Self::load_path(&candidate)?;
                return Ok((candidate, manifest));
            }
            if !dir.pop() {
                break;
            }
        }
        Err(ManifestError::NotFound {
            start: start.display().to_string(),
        })
    }

    pub fn is_verified(&self) -> bool {
        self.protocol_verified
            && self.data_version.is_some()
            && self.confidence == "verified"
            && !self.verified_on.trim().is_empty()
    }

    pub fn warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.protocol_verified {
            out.push(format!(
                "protocol_version {} is unverified; do not treat it as a code constant",
                self.protocol_version
            ));
        }
        if self.data_version.is_none() {
            out.push(
                "data_version is unset until extracted from installed Minecraft Java Edition 26.3"
                    .to_string(),
            );
        }
        if self.confidence != "verified" {
            out.push(format!(
                "version manifest confidence is `{}`",
                self.confidence
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
game_version = "26.3"
protocol_version = 777
protocol_verified = false
source = "pending"
verified_on = ""
confidence = "unverified"
dependent_modules = []
fixture_ids = []
"#;

    #[test]
    fn parse_unverified_26_3() {
        let m = VersionManifest::parse(SAMPLE).unwrap();
        assert_eq!(m.game_version, "26.3");
        assert_eq!(m.protocol_version, 777);
        assert!(m.data_version.is_none());
        assert!(!m.is_verified());
        assert_eq!(m.warnings().len(), 3);
    }

    #[test]
    fn rejects_empty_game_version() {
        let text = "game_version = \"\"\nprotocol_version = 1\n";
        assert!(matches!(
            VersionManifest::parse(text),
            Err(ManifestError::EmptyGameVersion)
        ));
    }
}
