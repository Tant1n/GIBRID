// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Текстурный атлас и интерфейс приёма текстур от лаунчера.
//! Лаунчер подгружает PNG из основного майна и передаёт в gibrid готовый атлас.
//! Статус: скелет полноценного ядра. Поддерживает автоматическую сборку атласа
//! из директории с PNG текстурой блоков.

use gibrid_core::ResourceId;
use image::DynamicImage;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

/// Один регион текстуры в атласе (UV-координаты).
#[derive(Debug, Clone, Copy)]
pub struct TextureRegion {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

/// Текстурный атлас: все блоковые текстуры в одной карте.
/// Данные `raw_rgba` передаются лаунчером, gibrid только хранит и отдаёт UV.
#[derive(Debug, Clone)]
pub struct TextureAtlas {
    pub width: u32,
    pub height: u32,
    pub regions: HashMap<String, TextureRegion>,
    pub raw_rgba: Vec<u8>,
}

impl TextureAtlas {
    pub fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            regions: HashMap::new(),
            raw_rgba: Vec::new(),
        }
    }

    pub fn region(&self, id: &ResourceId) -> Option<TextureRegion> {
        self.regions.get(&id.to_string()).copied()
    }
}

/// Интерфейс для лаунчера: передать готовый атлас в gibrid.
pub trait TextureProvider {
    fn atlas(&self) -> &TextureAtlas;
}

impl Default for TextureAtlas {
    fn default() -> Self {
        Self::empty()
    }
}

/// Загрузить все PNG текстур блоков из директории.
/// Ожидается структура: <dir>/<name>.png (например, textures/block/stone.png).
/// namespace передаётся отдельно, так как он одинаков для всего каталога (обычно "minecraft").
pub fn load_block_textures_from_dir(dir: &Path, namespace: &str) -> Result<HashMap<ResourceId, DynamicImage>, Box<dyn std::error::Error>> {
    let mut textures = HashMap::new();

    if !dir.is_dir() {
        return Ok(textures);
    }

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|e| format!("walkdir error: {e}"))?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "png") {
            let rel = path.strip_prefix(dir).map_err(|e| format!("strip_prefix: {e}"))?;
            let name = rel.to_string_lossy().trim_end_matches(".png").replace("\\", "/");
            let id = ResourceId::new(namespace, name.clone()).map_err(|e| format!("invalid resource id {namespace}:{name}: {e}"))?;
            let img = image::open(path)?;
            textures.insert(id, img);
        }
    }

    Ok(textures)
}

/// Построить текстурывый атлас из загруженных изображений.
/// Все текстуры приводятся к одному размеру (tile_size x tile_size).
/// tile_size по умолчанию 16 (стандартный размер блоковых текстур Minecraft).
pub fn build_atlas_from_images(
    textures: &HashMap<ResourceId, DynamicImage>,
    tile_size: u32,
) -> Result<TextureAtlas, Box<dyn std::error::Error>> {
    if textures.is_empty() {
        return Ok(TextureAtlas::empty());
    }

    let mut ids: Vec<&ResourceId> = textures.keys().collect();
    ids.sort_by(|a, b| a.to_string().cmp(&b.to_string()));

    let cols = (ids.len() as f32).sqrt().ceil() as u32;
    let rows = (ids.len() as u32 + cols - 1) / cols;
    let width = cols * tile_size;
    let height = rows * tile_size;
    let mut raw_rgba = vec![0u8; (width * height * 4) as usize];
    let mut regions = HashMap::new();

    for (i, id) in ids.iter().enumerate() {
        let cx = i as u32 % cols;
        let cy = i as u32 / cols;
        let ox = cx * tile_size;
        let oy = cy * tile_size;

        let img = textures.get(*id).unwrap();
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();

        // Копируем píксели в атлас (масштабируем под tile_size если нужно)
        for y in 0..h.min(tile_size) {
            for x in 0..w.min(tile_size) {
                let px = rgba.get_pixel(x, y);
                let ax = ox + x;
                let ay = oy + y;
                let idx = ((ay * width + ax) * 4) as usize;
                raw_rgba[idx] = px[0];
                raw_rgba[idx + 1] = px[1];
                raw_rgba[idx + 2] = px[2];
                raw_rgba[idx + 3] = px[3];
            }
        }

        let uv_min = [ox as f32 / width as f32, oy as f32 / height as f32];
        let uv_max = [((ox + tile_size) as f32) / width as f32, ((oy + tile_size) as f32) / height as f32];
        regions.insert(id.to_string(), TextureRegion {
            uv_min,
            uv_max,
        });
    }

    Ok(TextureAtlas {
        width,
        height,
        regions,
        raw_rgba,
    })
}

/// Загрузить текстуры блоков из директории gibrid_assets и собрать атлас.
/// Удобная обёртка для использования в gibrid-app.
pub fn load_and_build_atlas(
    assets_dir: &Path,
    tile_size: u32,
) -> Result<TextureAtlas, Box<dyn std::error::Error>> {
    let textures_dir = assets_dir.join("assets").join("minecraft").join("textures").join("block");
    let textures = load_block_textures_from_dir(&textures_dir, "minecraft")?;
    build_atlas_from_images(&textures, tile_size)
}

/// Загрузить текстурный атлас из извлечённого minecraft.jar (структура: assets/minecraft/textures/block/).
pub fn load_atlas_from_extracted_minecraft(
    minecraft_dir: &Path,
    tile_size: u32,
) -> Result<TextureAtlas, Box<dyn std::error::Error>> {
    let textures_dir = minecraft_dir.join("assets").join("minecraft").join("textures").join("block");
    let textures = load_block_textures_from_dir(&textures_dir, "minecraft")?;
    build_atlas_from_images(&textures, tile_size)
}

/// Загрузить текстурный атлас из реальных ассетов Minecraft (структура лаунчера).
///
/// Ожидается структура:
///   <assets_dir>/indexes/<version>.json   — объект "objects": {"path": {"hash": "...", "size": N}}
///   <assets_dir>/objects/<xx>/<hash>      — само содержимое файла
///
/// Берётся первый найденный индекс, из которого извлекаются все текстуры
/// с путями `minecraft/textures/block/*.png`.
pub fn load_atlas_from_asset_index(
    assets_dir: &Path,
    tile_size: u32,
) -> Result<TextureAtlas, Box<dyn std::error::Error>> {
    let indexes_dir = assets_dir.join("indexes");
    let objects_dir = assets_dir.join("objects");

    if !indexes_dir.is_dir() {
        return Err(format!("indexes dir not found: {}", indexes_dir.display()).into());
    }
    if !objects_dir.is_dir() {
        return Err(format!("objects dir not found: {}", objects_dir.display()).into());
    }

    // Найти индекс-файл (берём любой .json в папке indexes)
    let index_files: Vec<_> = fs::read_dir(&indexes_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
        .collect();

    if index_files.is_empty() {
        return Err("no index files found in assets/indexes".into());
    }

    // Пробуем каждый индекс, пока не найдём тот, в котором есть блоковые текстуры
    let mut best_textures: Option<HashMap<ResourceId, DynamicImage>> = None;

    for entry in index_files {
        let index_path = entry.path();
        let index_text = fs::read_to_string(&index_path)?;
        let index_value: serde_json::Value = serde_json::from_str(&index_text)?;

        let objects = index_value.get("objects").and_then(serde_json::Value::as_object);
        let Some(objects) = objects else {
            continue;
        };

        let mut textures = HashMap::new();
        for (logical_path, obj) in objects {
            if !logical_path.starts_with("minecraft/textures/block/") {
                continue;
            }
            if !logical_path.ends_with(".png") {
                continue;
            }

            let hash = obj.get("hash").and_then(serde_json::Value::as_str);
            let Some(hash) = hash else {
                continue;
            };

            let prefix = &hash[..2];
            let object_path = objects_dir.join(prefix).join(hash);

            if !object_path.exists() {
                continue;
            }

            let mut file = std::fs::File::open(&object_path)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;

            let img = match image::load_from_memory(&data) {
                Ok(img) => img,
                Err(_) => continue, // не PNG, пропускаем
            };

            let path_part = logical_path
                .strip_prefix("minecraft/textures/block/")
                .unwrap_or(logical_path)
                .trim_end_matches(".png");

            let id = ResourceId::new("minecraft", path_part)
                .map_err(|e| format!("invalid resource id minecraft:{path_part}: {e}"))?;

            textures.insert(id, img);
        }

        if !textures.is_empty() {
            best_textures = Some(textures);
            break;
        }
    }

    let textures = best_textures.ok_or("no block textures found in any index")?;
    build_atlas_from_images(&textures, tile_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_empty() {
        let atlas = TextureAtlas::empty();
        assert_eq!(atlas.width, 0);
        assert!(atlas.regions.is_empty());
        assert!(atlas.region(&gibrid_core::ResourceId::minecraft("stone").unwrap()).is_none());
    }

    #[test]
    fn build_atlas_from_single_texture() {
        use image::RgbaImage;
        let mut img = RgbaImage::new(16, 16);
        for y in 0..16 {
            for x in 0..16 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        let id = gibrid_core::ResourceId::minecraft("test_block").unwrap();
        let mut textures = HashMap::new();
        textures.insert(id.clone(), DynamicImage::ImageRgba8(img));

        let atlas = build_atlas_from_images(&textures, 16).unwrap();
        assert_eq!(atlas.width, 16);
        assert_eq!(atlas.height, 16);
        assert!(atlas.region(&id).is_some());
    }
}
