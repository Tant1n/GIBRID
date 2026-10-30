//! NBT and Anvil region (.mca) codecs with allocation limits.
//! Chunk sections and block-state palettes (gzip/zlib compressed payloads).

#![forbid(unsafe_code)]

pub mod chunk;
pub mod nbt;
pub mod region;

pub use chunk::{
    bits_per_block_for_palette, BlockStateEntry, BlockStatePalette, ChunkError, ChunkSection,
};
pub use nbt::{encode_root, parse_root, Limits, NbtError, Tag};
pub use region::{
    encode_chunk_payload, index_for_chunk, read_chunk, write_chunk_to_region,
    write_region_file_atomic, Compression, RegionError, RegionHeader, RawChunk, SectorLocation,
};
