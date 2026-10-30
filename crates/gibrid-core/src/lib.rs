//! Shared identifiers, coordinates, configuration, and version contracts.
//!
//! This crate does not own world simulation or GPU resources.

#![forbid(unsafe_code)]

pub mod config;
pub mod coords;
pub mod error;
pub mod ident;
pub mod lifecycle;
pub mod manifest;
pub mod tick;

pub use config::{AppConfig, ConfigError};
pub use coords::{block_to_chunk, block_to_local, block_to_section, chunk_to_region};
pub use error::CoreError;
pub use ident::ResourceId;
pub use lifecycle::Lifecycle;
pub use manifest::{ManifestError, VersionManifest};
pub use tick::{TickClock, TICKS_PER_SECOND, TICK_DURATION_MS};
