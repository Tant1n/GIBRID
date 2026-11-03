// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.//! Resource-pack path safety, discovery, precedence and cache invalidation.
//! Parsing of blockstates and block models.
//! Does not load Minecraft assets.

#![forbid(unsafe_code)]

pub mod asset_index;
pub mod block_definition;
pub mod block_model;
pub mod blockstate;
pub mod item_definition;
pub mod model;
pub mod sound_system;
pub mod texture;

use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use gibrid_core::ResourceId;
use gibrid_core::ident::ResourceId as GibridResourceId;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AssetError {
    #[error("resource path escapes the pack root: {0}")]
    PathTraversal(String),
    #[error("resource path is empty")]
    Empty,
    #[error("resource pack not found: {0}")]
    PackNotFound(String),
    #[error("resource not found in any pack: {0}")]
    ResourceNotFound(String),
    #[error("failed to read pack directory: {0}")]
    PackDir(String),
    #[error(transparent)]
    Ident(#[from] gibrid_core::ident::IdentError),
}

/// Resolve `namespace:path` under `pack_root/assets/namespace/path`.
pub fn resolve_in_pack(pack_root: &Path, id: &ResourceId) -> Result<PathBuf, AssetError> {
    if id.path().is_empty() {
        return Err(AssetError::Empty);
    }
    let mut out = pack_root.join("assets").join(id.namespace());
    for segment in id.path().split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(AssetError::PathTraversal(id.to_string()));
        }
        out.push(segment);
    }
    if out.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(AssetError::PathTraversal(id.to_string()));
    }
    Ok(out)
}

/// Description of a single resource pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourcePack {
    pub name: String,
    pub root: PathBuf,
    /// Higher value = higher priority. Packs are searched in descending priority order.
    pub priority: u32,
    /// Last modification time of the pack root directory, for cache invalidation.
    pub last_modified: Option<SystemTime>,
}

/// Manager for resource packs. Resolves resources by searching packs in priority order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackManager {
    packs: Vec<ResourcePack>,
}

impl Default for PackManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PackManager {
    pub fn new() -> Self {
        Self { packs: Vec::new() }
    }

    pub fn with_packs(mut packs: Vec<ResourcePack>) -> Self {
        packs.sort_by_key(|a| std::cmp::Reverse(a.priority));
        Self { packs }
    }

    pub fn add_pack(&mut self, pack: ResourcePack) {
        self.packs.push(pack);
        self.packs.sort_by_key(|a| std::cmp::Reverse(a.priority));
    }

    /// Remove a pack by name.
    pub fn remove_pack(&mut self, name: &str) {
        self.packs.retain(|p| p.name != name);
    }

    /// List pack names in priority order (highest first).
    pub fn pack_names(&self) -> Vec<String> {
        self.packs.iter().map(|p| p.name.clone()).collect()
    }

    /// Resolve a resource id by searching packs in priority order.
    /// Returns the first existing file path.
    pub fn resolve_resource(&self, id: &ResourceId) -> Result<PathBuf, AssetError> {
        for pack in &self.packs {
            let path = resolve_in_pack(&pack.root, id)?;
            if path.exists() {
                return Ok(path);
            }
        }
        Err(AssetError::ResourceNotFound(id.to_string()))
    }

    /// Build a cache key for a pack: name + last_modified timestamp.
    /// If the pack changes, the key changes and caches should be invalidated.
    pub fn cache_key(&self, pack_name: &str) -> Option<String> {
        self.packs.iter().find(|p| p.name == pack_name).map(|p| {
            let ts = p
                .last_modified
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("{}:{}", p.name, ts)
        })
    }

    /// Invalidate pack cache entries whose last_modified changed.
    /// Returns names of packs that were updated.
    pub fn check_invalidation(&self, previous: &PackManager) -> Vec<String> {
        let mut changed = Vec::new();
        for pack in &self.packs {
            let prev = previous.packs.iter().find(|p| p.name == pack.name);
            match prev {
                Some(p) if p.last_modified == pack.last_modified => {}
                _ => changed.push(pack.name.clone()),
            }
        }
        changed
    }
}

/// Discover resource packs in a directory.
/// Each subdirectory of `dir` is treated as a pack named after the directory.
/// Priority is assigned by filename sort order: first found = lowest priority,
/// later found = higher priority.
pub fn discover_packs(dir: &Path) -> Result<Vec<ResourcePack>, AssetError> {
    if !dir.is_dir() {
        return Err(AssetError::PackDir(format!(
            "not a directory: {}",
            dir.display()
        )));
    }

    let mut packs = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| AssetError::PackDir(e.to_string()))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            let last_modified = entry.metadata().and_then(|m| m.modified()).ok();
            packs.push(ResourcePack {
                name,
                root: path,
                priority: i as u32,
                last_modified,
            });
        }
    }

    // Reverse so that later-sorted directories get higher priority.
    packs.reverse();
    let total = packs.len();
    for (i, pack) in packs.iter_mut().enumerate() {
        pack.priority = (total - i) as u32;
    }

    Ok(packs)
}

/// Авто-подкачка ассетов из установленного Minecraft для тестов gibrid.
/// Копирует `assets/` и `resourcepacks/` из `.minecraft` в `<cwd>/gibrid_assets/`.
/// Это временное решение для разработки — в релизе ресурсы должны поставляться отдельно.
pub fn fetch_from_minecraft(minecraft_dir: &Path) -> Result<(), AssetError> {
    let target_dir = std::env::current_dir()
        .map_err(|e| AssetError::PackDir(e.to_string()))?
        .join("gibrid_assets");

    let source_assets = minecraft_dir.join("assets");
    let source_packs = minecraft_dir.join("resourcepacks");

    if source_assets.is_dir() {
        let target_assets = target_dir.join("assets");
        tracing::info!(
            source = %source_assets.display(),
            target = %target_assets.display(),
            "копируем ассеты из minecraft"
        );
        copy_dir_recursive(&source_assets, &target_assets)?;
    } else {
        tracing::warn!(
            source = %source_assets.display(),
            mc_dir = %minecraft_dir.display(),
            "папка assets не найдена в minecraft"
        );
    }

    if source_packs.is_dir() {
        let target_packs = target_dir.join("resourcepacks");
        tracing::info!(
            source = %source_packs.display(),
            target = %target_packs.display(),
            "копируем resourcepacks из minecraft"
        );
        copy_dir_recursive(&source_packs, &target_packs)?;
    }

    println!(
        "Ассеты скопированы в {}. Теперь gibrid может использовать текстуры и модели из основного майна.",
        target_dir.display()
    );
    Ok(())
}

/// Рекурсивное копирование директории.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), AssetError> {
    if !src.is_dir() {
        return Err(AssetError::PackDir(format!("источник не является директорией: {}", src.display())));
    }
    std::fs::create_dir_all(dst).map_err(|e| AssetError::PackDir(e.to_string()))?;

    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.map_err(|e| AssetError::PackDir(e.to_string()))?;
        let rel = entry.path().strip_prefix(src).map_err(|e| AssetError::PackDir(e.to_string()))?;
        let target = dst.join(rel);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| AssetError::PackDir(e.to_string()))?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| AssetError::PackDir(e.to_string()))?;
            }
            std::fs::copy(entry.path(), &target).map_err(|e| AssetError::PackDir(e.to_string()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::str::FromStr;

    #[test]
    fn joins_namespace_and_path() {
        let id = ResourceId::from_str("minecraft:block/stone").unwrap();
        let path = resolve_in_pack(Path::new("pack"), &id).unwrap();
        assert_eq!(
            path,
            Path::new("pack")
                .join("assets")
                .join("minecraft")
                .join("block")
                .join("stone")
        );
    }

    #[test]
    fn resource_id_rejects_dotdot() {
        assert!(ResourceId::from_str("minecraft:../stone").is_err());
    }

    #[test]
    fn pack_manager_priority_order() {
        let low = ResourcePack {
            name: "low".to_string(),
            root: Path::new("low").to_path_buf(),
            priority: 1,
            last_modified: None,
        };
        let high = ResourcePack {
            name: "high".to_string(),
            root: Path::new("high").to_path_buf(),
            priority: 10,
            last_modified: None,
        };
        let mgr = PackManager::with_packs(vec![low.clone(), high.clone()]);
        assert_eq!(mgr.pack_names(), vec!["high", "low"]);
    }

    #[test]
    fn pack_manager_resolves_highest_priority_first() {
        let a = ResourcePack {
            name: "a".to_string(),
            root: Path::new("a").to_path_buf(),
            priority: 5,
            last_modified: None,
        };
        let b = ResourcePack {
            name: "b".to_string(),
            root: Path::new("b").to_path_buf(),
            priority: 10,
            last_modified: None,
        };
        let mgr = PackManager::with_packs(vec![a, b]);
        assert_eq!(mgr.pack_names(), vec!["b", "a"]);
    }

    #[test]
    fn cache_key_changes_on_modification() {
        let mut pack = ResourcePack {
            name: "test".to_string(),
            root: Path::new("test").to_path_buf(),
            priority: 1,
            last_modified: Some(UNIX_EPOCH),
        };
        let mgr = PackManager::with_packs(vec![pack.clone()]);
        let key1 = mgr.cache_key("test").unwrap();

        pack.last_modified = Some(SystemTime::now());
        let mgr2 = PackManager::with_packs(vec![pack]);
        let key2 = mgr2.cache_key("test").unwrap();

        assert_ne!(key1, key2);
    }

    #[test]
    fn check_invalidation_detects_changes() {
        let unchanged = ResourcePack {
            name: "unchanged".to_string(),
            root: Path::new("u").to_path_buf(),
            priority: 1,
            last_modified: Some(UNIX_EPOCH),
        };
        let changed = ResourcePack {
            name: "changed".to_string(),
            root: Path::new("c").to_path_buf(),
            priority: 2,
            last_modified: Some(UNIX_EPOCH),
        };
        let old = PackManager::with_packs(vec![unchanged.clone(), changed.clone()]);

        let mut new_changed = changed.clone();
        new_changed.last_modified = Some(SystemTime::now());
        let new = PackManager::with_packs(vec![unchanged, new_changed]);

        let invalidated = new.check_invalidation(&old);
        assert_eq!(invalidated, vec!["changed".to_string()]);
    }
}

