// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.
use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// Namespaced identifier (`minecraft:stone`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId {
    namespace: String,
    path: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdentError {
    #[error("resource id is empty")]
    Empty,
    #[error("resource id `{0}` has an empty namespace or path")]
    EmptyPart(String),
    #[error("resource id `{0}` contains invalid characters")]
    InvalidChars(String),
    #[error("resource id `{0}` has more than one ':' separator")]
    ExtraSeparator(String),
}

impl ResourceId {
    pub fn new(namespace: impl Into<String>, path: impl Into<String>) -> Result<Self, IdentError> {
        let namespace = namespace.into();
        let path = path.into();
        let rendered = format!("{namespace}:{path}");
        validate(&namespace, &path, &rendered)?;
        Ok(Self { namespace, path })
    }

    pub fn minecraft(path: impl Into<String>) -> Result<Self, IdentError> {
        Self::new("minecraft", path)
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl FromStr for ResourceId {
    type Err = IdentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(IdentError::Empty);
        }
        match s.split_once(':') {
            None => Self::minecraft(s),
            Some((namespace, path)) => {
                if path.contains(':') {
                    return Err(IdentError::ExtraSeparator(s.to_string()));
                }
                Self::new(namespace, path)
            }
        }
    }
}

fn validate(namespace: &str, path: &str, rendered: &str) -> Result<(), IdentError> {
    if namespace.is_empty() || path.is_empty() {
        return Err(IdentError::EmptyPart(rendered.to_string()));
    }
    if !is_namespace(namespace) || !is_path(path) {
        return Err(IdentError::InvalidChars(rendered.to_string()));
    }
    Ok(())
}

fn is_namespace(value: &str) -> bool {
    value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' || c == '.')
}

fn is_path(value: &str) -> bool {
    for segment in value.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return false;
        }
        if !segment.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' || c == '.'
        }) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minecraft_default() {
        let id: ResourceId = "stone".parse().unwrap();
        assert_eq!(id.namespace(), "minecraft");
        assert_eq!(id.path(), "stone");
        assert_eq!(id.to_string(), "minecraft:stone");
    }

    #[test]
    fn rejects_traversal_like_and_uppercase() {
        assert!("minecraft:../secret".parse::<ResourceId>().is_err());
        assert!("MC:stone".parse::<ResourceId>().is_err());
        assert!(":stone".parse::<ResourceId>().is_err());
        assert!("a:b:c".parse::<ResourceId>().is_err());
    }
}

