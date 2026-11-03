// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.//! Block, chunk, and dimension coordinates. No simulation yet.

#![forbid(unsafe_code)]

use gibrid_core::coords::{
    block_to_chunk, block_to_local, block_to_section, chunk_to_local_in_region, chunk_to_region,
};
use gibrid_core::ResourceId;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn chunk(self) -> ChunkPos {
        ChunkPos::new(block_to_chunk(self.x), block_to_chunk(self.z))
    }

    pub fn section_y(self) -> i32 {
        block_to_section(self.y)
    }

    pub fn local(self) -> LocalPos {
        LocalPos {
            x: block_to_local(self.x) as u8,
            y: block_to_local(self.y) as u8,
            z: block_to_local(self.z) as u8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalPos {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub fn region(self) -> RegionPos {
        RegionPos::new(chunk_to_region(self.x), chunk_to_region(self.z))
    }

    pub fn index_in_region(self) -> u16 {
        let lx = chunk_to_local_in_region(self.x);
        let lz = chunk_to_local_in_region(self.z);
        (lx + lz * 32) as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionPos {
    pub x: i32,
    pub z: i32,
}

impl RegionPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DimensionId(ResourceId);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorldError {
    #[error(transparent)]
    Ident(#[from] gibrid_core::ident::IdentError),
}

impl DimensionId {
    pub fn overworld() -> Self {
        Self(ResourceId::minecraft("overworld").expect("static id"))
    }

    pub fn parse(id: &str) -> Result<Self, WorldError> {
        Ok(Self(id.parse()?))
    }

    pub fn as_resource(&self) -> &ResourceId {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_negatives_map_to_chunk_and_region() {
        let pos = BlockPos::new(-17, -17, -1);
        assert_eq!(pos.chunk(), ChunkPos::new(-2, -1));
        assert_eq!(
            pos.local(),
            LocalPos {
                x: 15,
                y: 15,
                z: 15
            }
        );
        assert_eq!(pos.chunk().index_in_region(), 30 + 31 * 32);
        assert_eq!(ChunkPos::new(-1, -1).index_in_region(), 1023);
    }
}

