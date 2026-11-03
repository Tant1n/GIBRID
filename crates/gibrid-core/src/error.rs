// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.
use thiserror::Error;

use crate::config::ConfigError;
use crate::ident::IdentError;
use crate::manifest::ManifestError;

/// Top-level typed error used by `gibrid-app` and libraries that need a shared envelope.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Ident(#[from] IdentError),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

