// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Чтение текстур и моделей из структуры ассетов современного лаунчера Minecraft.
//! Структура:
//!   .minecraft/assets/indexes/<version>.json   — объект "objects": {"path": {"hash": "...", "size": N}, ...}
//!   .minecraft/assets/objects/<xx>/<hash>      — само содержимое файла (xx — первые 2 символа хеша)
//!   .minecraft/assets/minecraft/models/block/*.json — блочные модели с UV

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Запись одного ассета в индексе.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

/// Индекс ассетов (содержимое файла indexes/<version>.json).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

/// Информация о найденной текстуре.
#[derive(Debug, Clone)]
pub struct AssetEntry {
    pub logical_path: String,   // например "minecraft:block/stone"
    pub hash: String,
    pub size: u64,
    pub object_path: PathBuf,   // полный путь к файлу в objects/
}

/// Загрузчик ассетов по индексу.
pub struct AssetIndexLoader {
    pub index: AssetIndex,
    pub objects_root: PathBuf,
}

impl AssetIndexLoader {
    /// Создать загрузчик из файла индекса и папки objects.
    pub fn new(index_path: &Path, objects_root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let index_data = fs::read_to_string(index_path)?;
        let index: AssetIndex = serde_json::from_str(&index_data)?;
        Ok(Self {
            index,
            objects_root: objects_root.to_path_buf(),
        })
    }

    /// Найти объект по логическому пути (например "minecraft:block/stone").
    pub fn find(&self, logical_path: &str) -> Option<AssetEntry> {
        let obj = self.index.objects.get(logical_path)?;
        let hash = &obj.hash;
        let prefix = &hash[..2];
        let object_path = self.objects_root.join(prefix).join(hash);
        if !object_path.exists() {
            return None;
        }
        Some(AssetEntry {
            logical_path: logical_path.to_string(),
            hash: hash.clone(),
            size: obj.size,
            object_path,
        })
    }

    /// Прочитать содержимое объекта по логическому пути.
    pub fn read(&self, logical_path: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        let entry = match self.find(logical_path) {
            Some(e) => e,
            None => return Ok(None),
        };
        let mut file = fs::File::open(&entry.object_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Ok(Some(data))
    }

    /// Перечислить все логические пути с заданным префиксом (например "minecraft:block/").
    pub fn list_prefix(&self, prefix: &str) -> Vec<String> {
        self.index
            .objects
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_and_find() {
        let dir = std::env::temp_dir().join("gibrid_asset_index_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("indexes")).unwrap();
        fs::create_dir_all(dir.join("objects").join("ab")).unwrap();

        // Пишем тестовый индекс
        let index = AssetIndex {
            objects: HashMap::from([
                ("minecraft:block/stone".to_string(), AssetObject { hash: "ab1234".to_string(), size: 100 }),
            ]),
        };
        let index_json = serde_json::to_string(&index).unwrap();
        fs::write(dir.join("indexes").join("test.json"), &index_json).unwrap();

        // Пишем тестовый объект
        let mut obj_file = fs::File::create(dir.join("objects").join("ab").join("ab1234")).unwrap();
        obj_file.write_all(b"fake texture data").unwrap();

        let loader = AssetIndexLoader::new(
            &dir.join("indexes").join("test.json"),
            &dir.join("objects"),
        ).unwrap();

        let entry = loader.find("minecraft:block/stone").unwrap();
        assert_eq!(entry.hash, "ab1234");
        assert_eq!(entry.logical_path, "minecraft:block/stone");

        let data = loader.read("minecraft:block/stone").unwrap().unwrap();
        assert_eq!(data, b"fake texture data");

        let missing = loader.read("minecraft:block/missing").unwrap();
        assert!(missing.is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
