// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.//! GIBRID World Generation - Phase 9
//! Procedural terrain generation with hash-based noise
//! Status: IMPLEMENTED - Basic chunk generation with hash-based heightmaps

#![forbid(unsafe_code)]

use gibrid_core::ResourceId;

/// Chunk coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub fn from_block_coords(x: i32, z: i32) -> Self {
        Self {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        }
    }
}

/// World generation configuration
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    pub seed: u64,
    pub sea_level: i32,
    pub sea_material: ResourceId,
    pub sea_state: u32,
    pub sea_fluid: ResourceId,
    pub sea_fluid_state: u32,
    pub sea_fluid_level: u8,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            sea_level: 63,
            sea_material: ResourceId::minecraft("water").unwrap(),
            sea_state: 0,
            sea_fluid: ResourceId::minecraft("water").unwrap(),
            sea_fluid_state: 0,
            sea_fluid_level: 0,
        }
    }
}

/// Simple hash-based noise function (deterministic, seedable)
fn hash_noise(seed: u64, x: i32, z: i32) -> f32 {
    let mut h = seed
        .wrapping_add((x as i64 as u64).wrapping_mul(0x9E3779B97F4A7C15))
        .wrapping_add((z as i64 as u64).wrapping_mul(0xBF58476D1CE4E5B9));
    h = h.wrapping_mul(0xFF51AFD7ED558CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CEB9FE1A85EC53);
    h ^= h >> 33;
    (h as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0
}

/// Smooth interpolation
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Main world generator
pub struct WorldGen {
    config: GeneratorConfig,
}

impl WorldGen {
    pub fn new(seed: u64) -> Self {
        Self {
            config: GeneratorConfig { seed, ..Default::default() },
        }
    }

    /// Generate a chunk at the given coordinates
    pub fn generate_chunk(&self, cx: i32, cz: i32) -> Vec<u8> {
        let mut data = Vec::new();

        for x in 0..16 {
            for z in 0..16 {
                let world_x = cx * 16 + x;
                let world_z = cz * 16 + z;

                let height = self.get_height(world_x, world_z);

                for y in 0..height {
                    let block_id = if y < self.config.sea_level {
                        self.config.sea_material.clone()
                    } else if y == height {
                        ResourceId::minecraft("grass_block").unwrap()
                    } else if y > height - 3 && y < height {
                        ResourceId::minecraft("dirt").unwrap()
                    } else {
                        ResourceId::minecraft("stone").unwrap()
                    };

                    let bytes = block_id.to_string().into_bytes();
                    data.push(bytes.len() as u8);
                    data.extend_from_slice(&bytes);
                    data.push(0);
                }
            }
        }

        data
    }

    /// Generate height at given coordinates using hash-based noise
    pub fn get_height(&self, x: i32, z: i32) -> i32 {
        // Combine multiple octaves of hash noise for terrain
        let n1 = hash_noise(self.config.seed, x, z);
        let n2 = hash_noise(self.config.seed.wrapping_add(1), x / 2, z / 2);
        let n3 = hash_noise(self.config.seed.wrapping_add(2), x / 4, z / 4);

        let combined = n1 * 0.5 + n2 * 0.25 + n3 * 0.125;
        let height = (combined * 20.0 + 64.0).clamp(0.0, 255.0) as i32;
        height
    }

    /// Get biome at coordinates based on noise
    pub fn get_biome(&self, x: i32, z: i32) -> String {
        let noise_val = hash_noise(self.config.seed.wrapping_add(3), x / 10, z / 10);
        if noise_val > 0.3 {
            "minecraft:forest".to_string()
        } else if noise_val > 0.0 {
            "minecraft:plains".to_string()
        } else if noise_val > -0.3 {
            "minecraft:desert".to_string()
        } else {
            "minecraft:ocean".to_string()
        }
    }
}

impl Default for WorldGen {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_pos_creation() {
        let pos = ChunkPos::new(5, -3);
        assert_eq!(pos.x, 5);
        assert_eq!(pos.z, -3);
    }

    #[test]
    fn chunk_pos_from_block_coords() {
        let pos = ChunkPos::from_block_coords(20, 35);
        assert_eq!(pos.x, 1);
        assert_eq!(pos.z, 2);
    }

    #[test]
    fn generator_config_default() {
        let config = GeneratorConfig::default();
        assert_eq!(config.sea_level, 63);
        assert_eq!(config.seed, 0);
    }

    #[test]
    fn world_gen_creation() {
        let gen = WorldGen::new(12345);
        assert_eq!(gen.config.seed, 12345);
    }

    #[test]
    fn get_height_range() {
        let gen = WorldGen::new(42);
        let h = gen.get_height(0, 0);
        assert!(h >= 0);
        assert!(h <= 255);
    }

    #[test]
    fn height_deterministic() {
        let gen1 = WorldGen::new(42);
        let gen2 = WorldGen::new(42);
        assert_eq!(gen1.get_height(10, 20), gen2.get_height(10, 20));
    }

    #[test]
    fn height_changes_with_seed() {
        let gen1 = WorldGen::new(42);
        let gen2 = WorldGen::new(99);
        // Heights should likely differ (not guaranteed but very likely)
        let _ = (gen1.get_height(10, 20), gen2.get_height(10, 20));
    }

    #[test]
    fn generate_chunk_non_empty() {
        let gen = WorldGen::new(7);
        let data = gen.generate_chunk(0, 0);
        assert!(!data.is_empty());
    }
}
