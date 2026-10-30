//! Minecraft-style integer coordinate conversions.
//!
//! Division uses Euclidean rounding so negative coordinates stay consistent
//! with chunk/section/region grids. See GIBRID_DEV_GUIDE_CORE_EXAMPLES.

pub const CHUNK_SIZE: i32 = 16;
pub const REGION_CHUNKS: i32 = 32;

/// Convert a block coordinate on one axis into a chunk coordinate.
pub fn block_to_chunk(block: i32) -> i32 {
    block.div_euclid(CHUNK_SIZE)
}

/// Convert a block coordinate on one axis into 0..=15 inside its chunk.
pub fn block_to_local(block: i32) -> i32 {
    block.rem_euclid(CHUNK_SIZE)
}

/// Convert a block Y coordinate into a section index.
pub fn block_to_section(block_y: i32) -> i32 {
    block_y.div_euclid(CHUNK_SIZE)
}

/// Convert a chunk coordinate on one axis into a region coordinate.
pub fn chunk_to_region(chunk: i32) -> i32 {
    chunk.div_euclid(REGION_CHUNKS)
}

/// Convert a chunk coordinate into 0..=31 inside its region.
pub fn chunk_to_local_in_region(chunk: i32) -> i32 {
    chunk.rem_euclid(REGION_CHUNKS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_and_boundary_block_to_chunk() {
        let cases = [
            (-17, -2, 15),
            (-16, -1, 0),
            (-1, -1, 15),
            (0, 0, 0),
            (15, 0, 15),
            (16, 1, 0),
        ];
        for (block, chunk, local) in cases {
            assert_eq!(block_to_chunk(block), chunk, "chunk for {block}");
            assert_eq!(block_to_local(block), local, "local for {block}");
            assert_eq!(chunk * CHUNK_SIZE + local, block, "round-trip for {block}");
        }
    }

    #[test]
    fn section_follows_euclidean_y() {
        assert_eq!(block_to_section(-17), -2);
        assert_eq!(block_to_section(-16), -1);
        assert_eq!(block_to_section(0), 0);
        assert_eq!(block_to_section(16), 1);
    }

    #[test]
    fn region_boundaries() {
        assert_eq!(chunk_to_region(-33), -2);
        assert_eq!(chunk_to_region(-32), -1);
        assert_eq!(chunk_to_region(-1), -1);
        assert_eq!(chunk_to_region(0), 0);
        assert_eq!(chunk_to_region(31), 0);
        assert_eq!(chunk_to_region(32), 1);
        assert_eq!(chunk_to_local_in_region(-1), 31);
        assert_eq!(chunk_to_local_in_region(-33), 31);
    }
}
