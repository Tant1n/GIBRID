// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Ядро симуляции: мир, чанки, блоки.
//! Статус: скелет полноценного ядра. Мир хранит чанки 16x16x16, отслеживает грязные чанки
//! для перемешивания и обновления мешей.

use crate::ResourceId;
use std::collections::HashMap;

pub type PaletteIndex = u16;

/// Координата чанка в мировых чанках.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkKey {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkKey {
    pub fn from_block(x: i32, y: i32, z: i32) -> Self {
        Self {
            x: x.div_euclid(16),
            y: y.div_euclid(16),
            z: z.div_euclid(16),
        }
    }
}

/// Один чанк 16x16x16 блоков с палитрой и освещением.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub key: ChunkKey,
    pub palette: Vec<ResourceId>,
    pub blocks: [PaletteIndex; 4096],
    pub block_light: [u8; 4096],   // 0..15, освещение от блоков (фонари, светящийся камень)
    pub sky_light: [u8; 4096],     // 0..15, небесное освещение
    pub dirty: bool,
}

impl Chunk {
    pub fn new(key: ChunkKey) -> Self {
        Self {
            key,
            palette: vec![ResourceId::minecraft("air").unwrap()],
            blocks: [0; 4096],
            block_light: [0; 4096],
            sky_light: [0; 4096],
            dirty: true,
        }
    }

    #[inline]
    pub fn index(lx: i32, ly: i32, lz: i32) -> usize {
        debug_assert!(lx >= 0 && lx < 16 && ly >= 0 && ly < 16 && lz >= 0 && lz < 16);
        ((ly * 16 + lz) * 16 + lx) as usize
    }

    pub fn get_block(&self, lx: i32, ly: i32, lz: i32) -> Option<ResourceId> {
        if lx < 0 || lx >= 16 || ly < 0 || ly >= 16 || lz < 0 || lz >= 16 {
            return None;
        }
        let idx = Self::index(lx, ly, lz);
        self.palette.get(self.blocks[idx] as usize).cloned()
    }

    pub fn set_block(&mut self, lx: i32, ly: i32, lz: i32, block: Option<ResourceId>) {
        if lx < 0 || lx >= 16 || ly < 0 || ly >= 16 || lz < 0 || lz >= 16 {
            return;
        }
        let idx = Self::index(lx, ly, lz);
        let pal_idx = match block {
            Some(b) => match self.palette.iter().position(|p| *p == b) {
                Some(p) => p as PaletteIndex,
                None => {
                    let ni = self.palette.len() as PaletteIndex;
                    self.palette.push(b);
                    ni
                }
            },
            None => 0,
        };
        if self.blocks[idx] != pal_idx {
            self.blocks[idx] = pal_idx;
            self.dirty = true;
        }
    }

    /// Установить блочное освещение (0..15) для позиции в чанке.
    pub fn set_block_light(&mut self, lx: i32, ly: i32, lz: i32, light: u8) {
        if lx < 0 || lx >= 16 || ly < 0 || ly >= 16 || lz < 0 || lz >= 16 {
            return;
        }
        self.block_light[Self::index(lx, ly, lz)] = light.min(15);
        self.dirty = true;
    }

    /// Установить небесное освещение (0..15) для позиции в чанке.
    pub fn set_sky_light(&mut self, lx: i32, ly: i32, lz: i32, light: u8) {
        if lx < 0 || lx >= 16 || ly < 0 || ly >= 16 || lz < 0 || lz >= 16 {
            return;
        }
        self.sky_light[Self::index(lx, ly, lz)] = light.min(15);
        self.dirty = true;
    }

    /// Получить итоговую яркость (max(block_light, sky_light)) 0..15.
    pub fn brightness(&self, lx: i32, ly: i32, lz: i32) -> u8 {
        if lx < 0 || lx >= 16 || ly < 0 || ly >= 16 || lz < 0 || lz >= 16 {
            return 0;
        }
        let idx = Self::index(lx, ly, lz);
        self.block_light[idx].max(self.sky_light[idx])
    }
}

/// Мир: набор загруженных чанков.
#[derive(Debug, Clone, Default)]
pub struct World {
    pub chunks: HashMap<ChunkKey, Chunk>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Option<ResourceId> {
        let key = ChunkKey::from_block(x, y, z);
        let chunk = self.chunks.get(&key)?;
        let lx = x.rem_euclid(16);
        let ly = y.rem_euclid(16);
        let lz = z.rem_euclid(16);
        chunk.get_block(lx, ly, lz)
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Option<ResourceId>) {
        let key = ChunkKey::from_block(x, y, z);
        let lx = x.rem_euclid(16);
        let ly = y.rem_euclid(16);
        let lz = z.rem_euclid(16);
        self.chunks
            .entry(key)
            .or_insert_with(|| Chunk::new(key))
            .set_block(lx, ly, lz, block);
    }

    pub fn dirty_chunks(&self) -> Vec<ChunkKey> {
        self.chunks
            .iter()
            .filter(|(_, c)| c.dirty)
            .map(|(k, _)| *k)
            .collect()
    }

    pub fn mark_clean(&mut self, key: &ChunkKey) {
        if let Some(c) = self.chunks.get_mut(key) {
            c.dirty = false;
        }
    }

    /// Получить блочное освещение (0..15) для мировых координат.
    pub fn get_block_light(&self, x: i32, y: i32, z: i32) -> Option<u8> {
        let key = ChunkKey::from_block(x, y, z);
        let chunk = self.chunks.get(&key)?;
        let lx = x.rem_euclid(16);
        let ly = y.rem_euclid(16);
        let lz = z.rem_euclid(16);
        Some(chunk.block_light[Chunk::index(lx, ly, lz)])
    }

    /// Получить небесное освещение (0..15) для мировых координат.
    pub fn get_sky_light(&self, x: i32, y: i32, z: i32) -> Option<u8> {
        let key = ChunkKey::from_block(x, y, z);
        let chunk = self.chunks.get(&key)?;
        let lx = x.rem_euclid(16);
        let ly = y.rem_euclid(16);
        let lz = z.rem_euclid(16);
        Some(chunk.sky_light[Chunk::index(lx, ly, lz)])
    }

    /// Получить итоговую яркость (max(block, sky)) 0..15.
    pub fn get_brightness(&self, x: i32, y: i32, z: i32) -> Option<u8> {
        let key = ChunkKey::from_block(x, y, z);
        let chunk = self.chunks.get(&key)?;
        let lx = x.rem_euclid(16);
        let ly = y.rem_euclid(16);
        let lz = z.rem_euclid(16);
        Some(chunk.brightness(lx, ly, lz))
    }

    /// Рассчитать небесное освещение для всего чанка (простая модель: чем выше, тем светлее,
    /// непрозрачные блоки загораживают свет снизу).
    pub fn calculate_sky_light(&mut self, key: ChunkKey) {
        let chunk = match self.chunks.get_mut(&key) {
            Some(c) => c,
            None => return,
        };

        for x in 0..16 {
            for z in 0..16 {
                let mut light = 15u8;
                // Идём сверху вниз, пока не встретим непрозрачный блок
                for y in (0..16).rev() {
                    let idx = Chunk::index(x, y, z);
                    let block_opt = chunk.palette.get(chunk.blocks[idx] as usize);
                    let is_opaque = block_opt.map_or(false, |b| !b.path().starts_with("air"));
                    if is_opaque {
                        light = 0;
                    } else if light > 0 {
                        light -= 1;
                    }
                    chunk.sky_light[idx] = light;
                }
            }
        }
        chunk.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_block() {
        let mut world = World::new();
        let stone = ResourceId::minecraft("stone").unwrap();
        world.set_block(0, 63, 0, Some(stone.clone()));
        assert_eq!(world.get_block(0, 63, 0), Some(stone));
        assert_eq!(world.get_block(0, 62, 0), None); // air не в палитре -> None
    }

    #[test]
    fn chunk_boundary() {
        let mut world = World::new();
        let stone = ResourceId::minecraft("stone").unwrap();
        world.set_block(16, 63, 0, Some(stone.clone()));
        assert_eq!(world.get_block(16, 63, 0), Some(stone));
        assert_eq!(world.get_block(15, 63, 0), None);
    }
}
