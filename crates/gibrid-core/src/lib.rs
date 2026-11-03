// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.//! Shared identifiers, coordinates, configuration, and version contracts.
//!
//! This crate does not own world simulation or GPU resources.

#![forbid(unsafe_code)]

pub mod accounts;
pub mod config;
pub mod coords;
pub mod entity;
pub mod error;
pub mod ident;
pub mod lifecycle;
pub mod manifest;
pub mod simulation;
pub mod tick;

pub use accounts::{AccountManager, AccountError, OfflineAccount};
pub use config::{AppConfig, ConfigError};
pub use coords::{block_to_chunk, block_to_local, block_to_section, chunk_to_region};
pub use entity::{AABB, Entity, EntityKind, EntityWorld, Physics};
pub use error::CoreError;
pub use ident::ResourceId;
pub use lifecycle::Lifecycle;
pub use manifest::{ManifestError, VersionManifest};
pub use simulation::{Chunk, ChunkKey, World};
pub use tick::{TickClock, TICKS_PER_SECOND, TICK_DURATION_MS};

