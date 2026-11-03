// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Блочные модели Minecraft из `assets/minecraft/models/block/*.json` и текстуры из
//! `assets/minecraft/textures/block/*.png`.
//! Парсит элементы куба с UV-развёртками и текстурами, готовые для мешинга.

use gibrid_core::ResourceId;
use image::DynamicImage;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Грани куба.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl BlockFace {
    pub fn all() -> [BlockFace; 6] {
        [
            BlockFace::Down,
            BlockFace::Up,
            BlockFace::North,
            BlockFace::South,
            BlockFace::West,
            BlockFace::East,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BlockFace::Down => "down",
            BlockFace::Up => "up",
            BlockFace::North => "north",
            BlockFace::South => "south",
            BlockFace::West => "west",
            BlockFace::East => "east",
        }
    }
}

/// Один элемент геометрии блока (куб или его часть).
#[derive(Debug, Clone, Deserialize)]
pub struct BlockElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub faces: Option<HashMap<String, BlockElementFace>>,
}

/// Описание грани элемента.
#[derive(Debug, Clone, Deserialize)]
pub struct BlockElementFace {
    pub uv: Option<[f32; 4]>, // [u1, v1, u2, v2] в пикселях текстуры
    pub texture: String,      // "#all" или "minecraft:block/stone"
}

/// Полная модель блока.
#[derive(Debug, Clone, Deserialize)]
pub struct BlockModel {
    pub parent: Option<String>,
    pub elements: Option<Vec<BlockElement>>,
    pub textures: Option<HashMap<String, String>>,
}

/// Загрузчик блочных моделей и текстур.
pub struct BlockModelLoader {
    models_dir: PathBuf,
    textures_dir: PathBuf,
    model_cache: HashMap<String, BlockModel>,
    texture_cache: HashMap<String, DynamicImage>,
}

impl BlockModelLoader {
    pub fn new(models_dir: &Path, textures_dir: &Path) -> Self {
        Self {
            models_dir: models_dir.to_path_buf(),
            textures_dir: textures_dir.to_path_buf(),
            model_cache: HashMap::new(),
            texture_cache: HashMap::new(),
        }
    }

    /// Загрузить модель блока по имени (например "minecraft:stone" -> "stone").
    pub fn load(&mut self, model_name: &str) -> Result<Option<BlockModel>, Box<dyn std::error::Error>> {
        if self.model_cache.contains_key(model_name) {
            return Ok(self.model_cache.get(model_name).cloned());
        }

        let short = model_name.strip_prefix("minecraft:").unwrap_or(model_name);
        let path = self.models_dir.join(format!("{}.json", short));

        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read_to_string(&path)?;
        let model: BlockModel = serde_json::from_str(&data)?;
        self.model_cache.insert(model_name.to_string(), model.clone());
        Ok(Some(model))
    }

    /// Загрузить текстуру по логическому пути (например "minecraft:block/stone").
    pub fn load_texture(&mut self, logical_path: &str) -> Result<Option<DynamicImage>, Box<dyn std::error::Error>> {
        if self.texture_cache.contains_key(logical_path) {
            return Ok(self.texture_cache.get(logical_path).cloned());
        }

        let path = self.textures_dir.join("block").join(format!("{}.png", logical_path));
        if !path.exists() {
            return Ok(None);
        }

        let img = image::open(&path)?;
        self.texture_cache.insert(logical_path.to_string(), img.clone());
        Ok(Some(img))
    }

    /// Загрузить текстуру для конкретной грани блока.
    pub fn load_face_texture(&mut self, model: &BlockModel, face_ref: &str) -> Result<Option<(ResourceId, DynamicImage)>, Box<dyn std::error::Error>> {
        let texture_ref = self.resolve_texture(model, face_ref).ok_or("texture reference not found")?;
        let texture_name = texture_ref.strip_prefix('#').unwrap_or(&texture_ref);
    
        // Извлечь имя блока из пути (например "minecraft:block/stone" -> "stone")
        let block_name = texture_name
            .strip_prefix("minecraft:block/")
            .unwrap_or(texture_name);
    
        let id = ResourceId::new("minecraft", block_name)?;
        let img = self.load_texture(&format!("minecraft:block/{}", block_name))?
            .ok_or_else(|| format!("texture not found: {}", block_name))?;
        Ok(Some((id, img)))
    }

    /// Разрешить ссылку на текстуру типа "#all" или "#side" в конкретное имя ресурса.
    pub fn resolve_texture(&self, model: &BlockModel, face_ref: &str) -> Option<String> {
        if let Some(rest) = face_ref.strip_prefix('#') {
            model.textures.as_ref().and_then(|t| t.get(rest).cloned())
        } else {
            Some(face_ref.to_string())
        }
    }
}

impl Default for BlockModelLoader {
    fn default() -> Self {
        Self::new(
            Path::new("gibrid_assets/assets/minecraft/models/block"),
            Path::new("gibrid_assets/assets/minecraft/textures"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_simple_model() {
        let dir = std::env::temp_dir().join("gibrid_block_model_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let model_json = r#"{
            "parent": "block/cube_all",
            "textures": {
                "all": "minecraft:block/stone"
            }
        }"#;
        fs::write(dir.join("stone.json"), model_json).unwrap();

        let mut loader = BlockModelLoader::new(&dir, Path::new("gibrid_assets/assets/minecraft/textures"));
        let model = loader.load("minecraft:stone").unwrap().unwrap();
        assert_eq!(model.parent.as_deref(), Some("block/cube_all"));
        assert_eq!(
            model.textures.as_ref().unwrap().get("all").unwrap(),
            "minecraft:block/stone"
        );

        let resolved = loader.resolve_texture(&model, "#all").unwrap();
        assert_eq!(resolved, "minecraft:block/stone");

        let _ = fs::remove_dir_all(&dir);
    }
}