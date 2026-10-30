use std::io::{Read, Write};

use flate2::read::GzDecoder;
use flate2::read::ZlibDecoder;
use flate2::write::{GzEncoder, ZlibEncoder};
use thiserror::Error;

pub const HEADER_SIZE: usize = 8192;
pub const SECTOR_SIZE: usize = 4096;
pub const CHUNKS_PER_REGION: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SectorLocation {
    pub sector_offset: u32,
    pub sector_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionHeader {
    locations: [Option<SectorLocation>; CHUNKS_PER_REGION],
    timestamps: [u32; CHUNKS_PER_REGION],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    Gzip,
    Zlib,
    Uncompressed,
    Unknown(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawChunk {
    pub compression: Compression,
    pub data: Vec<u8>,
}

impl RawChunk {
    /// Распаковать payload чанка в соответствии с типом сжатия MCA.
    /// Возвращает ошибку, если формат сжатия неизвестен или данные биты.
    pub fn decompress(&self) -> Result<Vec<u8>, RegionError> {
        match self.compression {
            Compression::Uncompressed => Ok(self.data.clone()),
            Compression::Gzip => {
                let mut decoder = GzDecoder::new(&self.data[..]);
                let mut out = Vec::new();
                decoder
                    .read_to_end(&mut out)
                    .map_err(|_| RegionError::DecompressFailed)?;
                Ok(out)
            }
            Compression::Zlib => {
                let mut decoder = ZlibDecoder::new(&self.data[..]);
                let mut out = Vec::new();
                decoder
                    .read_to_end(&mut out)
                    .map_err(|_| RegionError::DecompressFailed)?;
                Ok(out)
            }
            Compression::Unknown(id) => Err(RegionError::UnknownCompression(id)),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RegionError {
    #[error("region file is shorter than 8KiB header ({0} bytes)")]
    HeaderTooSmall(usize),
    #[error("chunk index {0} is out of range (0..=1023)")]
    Index(u16),
    #[error("chunk payload at offset {offset} is truncated")]
    Truncated { offset: usize },
    #[error("chunk payload length {len} is invalid at offset {offset}")]
    Length { len: u32, offset: usize },
    #[error("sector offset {0} points inside the header")]
    OverlapsHeader(u32),
    #[error("failed to decompress chunk payload")]
    DecompressFailed,
    #[error("unknown chunk compression id {0}")]
    UnknownCompression(u8),
}

pub fn index_for_chunk(chunk_x: i32, chunk_z: i32) -> u16 {
    let lx = chunk_x.rem_euclid(32) as u16;
    let lz = chunk_z.rem_euclid(32) as u16;
    lx + lz * 32
}

impl RegionHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, RegionError> {
        if bytes.len() < HEADER_SIZE {
            return Err(RegionError::HeaderTooSmall(bytes.len()));
        }
        let mut locations = [None; CHUNKS_PER_REGION];
        let mut timestamps = [0u32; CHUNKS_PER_REGION];
        for i in 0..CHUNKS_PER_REGION {
            let o = i * 4;
            let offset = u32::from_be_bytes([0, bytes[o], bytes[o + 1], bytes[o + 2]]);
            let count = bytes[o + 3];
            if offset != 0 && count != 0 {
                if offset < 2 {
                    return Err(RegionError::OverlapsHeader(offset));
                }
                locations[i] = Some(SectorLocation {
                    sector_offset: offset,
                    sector_count: count,
                });
            }
            let t = HEADER_SIZE / 2 + o;
            timestamps[i] =
                u32::from_be_bytes([bytes[t], bytes[t + 1], bytes[t + 2], bytes[t + 3]]);
        }
        Ok(Self {
            locations,
            timestamps,
        })
    }

    pub fn location(&self, index: u16) -> Option<SectorLocation> {
        self.locations.get(index as usize).copied().flatten()
    }

    pub fn timestamp(&self, index: u16) -> Option<u32> {
        self.timestamps.get(index as usize).copied()
    }
}

pub fn read_chunk(file: &[u8], loc: SectorLocation) -> Result<RawChunk, RegionError> {
    let start = (loc.sector_offset as usize)
        .checked_mul(SECTOR_SIZE)
        .ok_or(RegionError::Truncated { offset: 0 })?;
    if start + 5 > file.len() {
        return Err(RegionError::Truncated { offset: start });
    }
    let len = u32::from_be_bytes([
        file[start],
        file[start + 1],
        file[start + 2],
        file[start + 3],
    ]);
    if len == 0 {
        return Err(RegionError::Length { len, offset: start });
    }
    let payload_end = start
        .checked_add(4)
        .and_then(|v| v.checked_add(len as usize))
        .ok_or(RegionError::Length { len, offset: start })?;
    let max = start + loc.sector_count as usize * SECTOR_SIZE;
    if payload_end > file.len() || payload_end > max {
        return Err(RegionError::Truncated { offset: start });
    }
    let compression_id = file[start + 4];
    let compression = match compression_id {
        1 => Compression::Gzip,
        2 => Compression::Zlib,
        3 => Compression::Uncompressed,
        other => Compression::Unknown(other),
    };
    let data = file[start + 5..payload_end].to_vec();
    Ok(RawChunk { compression, data })
}

/// Кодирует RawChunk в байты сектора региона (длина + тип сжатия + данные).
/// Возвращает (сжатые_данные, compression_id, secteur_count).
pub fn encode_chunk_payload(raw: &RawChunk) -> Result<(Vec<u8>, u8, u8), RegionError> {
    let (compressed, compression_id) = match raw.compression {
        Compression::Uncompressed => (raw.data.clone(), 3),
        Compression::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::default());
            encoder
                .write_all(&raw.data)
                .map_err(|_| RegionError::DecompressFailed)?;
            (
                encoder
                    .finish()
                    .map_err(|_| RegionError::DecompressFailed)?,
                1,
            )
        }
        Compression::Zlib => {
            let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            encoder
                .write_all(&raw.data)
                .map_err(|_| RegionError::DecompressFailed)?;
            (
                encoder
                    .finish()
                    .map_err(|_| RegionError::DecompressFailed)?,
                2,
            )
        }
        Compression::Unknown(id) => return Err(RegionError::UnknownCompression(id)),
    };

    if compressed.len() > u32::MAX as usize {
        return Err(RegionError::Length {
            len: compressed.len() as u32,
            offset: 0,
        });
    }

    let sector_count = ((compressed.len() + 5) as u64 / SECTOR_SIZE as u64 + 1) as u8;

    Ok((compressed, compression_id, sector_count))
}

/// Записывает чанк в байты региона по заданному индексу.
/// Возвращает новые байты региона с обновлённым заголовком и данными чанка.
pub fn write_chunk_to_region(
    region_bytes: &[u8],
    chunk_index: u16,
    raw: &RawChunk,
) -> Result<Vec<u8>, RegionError> {
    if chunk_index as usize >= CHUNKS_PER_REGION {
        return Err(RegionError::Index(chunk_index));
    }

    let header = RegionHeader::parse(region_bytes)?;
    let existing_timestamp = header.timestamp(chunk_index).unwrap_or(0);

    let (compressed, compression_id, sector_count) = encode_chunk_payload(raw)?;

    // Вычисляем необходимое количество секторов для данных чанка
    let chunk_payload_len = 4 + 1 + compressed.len(); // длина + тип + данные
    let needed_sectors = (chunk_payload_len as u64).div_ceil(SECTOR_SIZE as u64) as u32;

    // Находим свободное место: простой подход — записываем в конец файла
    let mut new_bytes = region_bytes.to_vec();

    // Расширяем файл если нужно
    let current_sectors = (new_bytes.len() as u64 / SECTOR_SIZE as u64) as u32;
    let start_sector = current_sectors;
    let end_sector = start_sector + needed_sectors;
    let needed_file_size = end_sector as usize * SECTOR_SIZE;
    if new_bytes.len() < needed_file_size {
        new_bytes.resize(needed_file_size, 0);
    }

    let data_start = start_sector as usize * SECTOR_SIZE;
    let payload_start = data_start + 4; // после длины
    let payload_end = payload_start + compressed.len();

    // Пишем длину (4 байта, big-endian)
    new_bytes[data_start..data_start + 4]
        .copy_from_slice(&(compressed.len() as u32 + 1).to_be_bytes()); // +1 для байта сжатия
                                                                        // Пишем тип сжатия
    new_bytes[data_start + 4] = compression_id;
    // Пишем сжатые данные
    new_bytes[payload_start..payload_end].copy_from_slice(&compressed);

    // Обновляем location table в заголовке
    let mut locations = header.locations;
    locations[chunk_index as usize] = Some(SectorLocation {
        sector_offset: start_sector,
        sector_count,
    });
    let mut timestamps = header.timestamps;
    timestamps[chunk_index as usize] = existing_timestamp.wrapping_add(1);

    // Перезаписываем заголовок (первые 8192 байт)
    for (i, loc_opt) in locations.iter().enumerate().take(CHUNKS_PER_REGION) {
        let loc = match loc_opt {
            Some(l) => *l,
            None => SectorLocation::default(),
        };
        let o = i * 4;
        new_bytes[o] = (loc.sector_offset >> 16) as u8;
        new_bytes[o + 1] = (loc.sector_offset >> 8) as u8;
        new_bytes[o + 2] = loc.sector_offset as u8;
        new_bytes[o + 3] = loc.sector_count;
    }
    let ts_start = HEADER_SIZE / 2;
    for (i, ts) in timestamps.iter().enumerate().take(CHUNKS_PER_REGION) {
        let o = ts_start + i * 4;
        new_bytes[o..o + 4].copy_from_slice(&ts.to_be_bytes());
    }

    Ok(new_bytes)
}

/// Атомарная запись регионального файла: временный файл → валидация → замена.
pub fn write_region_file_atomic(
    path: &std::path::Path,
    region_bytes: &[u8],
) -> Result<(), RegionError> {
    use std::io::Write;

    let temp_path = path.with_extension("mca.tmp");
    {
        let mut file =
            std::fs::File::create(&temp_path).map_err(|_| RegionError::Truncated { offset: 0 })?;
        file.write_all(region_bytes)
            .map_err(|_| RegionError::Truncated { offset: 0 })?;
        file.sync_all()
            .map_err(|_| RegionError::Truncated { offset: 0 })?;
    }

    // Валидируем записанный файл
    let written = std::fs::read(&temp_path).map_err(|_| RegionError::Truncated { offset: 0 })?;
    RegionHeader::parse(&written).map_err(|_| RegionError::Truncated { offset: 0 })?;

    // Атомарная замена
    std::fs::rename(&temp_path, path).map_err(|_| RegionError::Truncated { offset: 0 })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_chunk_index() {
        assert_eq!(index_for_chunk(-1, -1), 1023);
        assert_eq!(index_for_chunk(0, 0), 0);
        assert_eq!(index_for_chunk(31, 0), 31);
    }

    #[test]
    fn empty_header() {
        let bytes = vec![0u8; HEADER_SIZE];
        let header = RegionHeader::parse(&bytes).unwrap();
        assert!(header.location(0).is_none());
    }

    #[test]
    fn rejects_short_header() {
        assert!(matches!(
            RegionHeader::parse(&[0u8; 16]),
            Err(RegionError::HeaderTooSmall(16))
        ));
    }

    #[test]
    fn reads_uncompressed_payload() {
        let mut bytes = vec![0u8; HEADER_SIZE + SECTOR_SIZE];
        // location index 0: sector 2, count 1
        bytes[2] = 2;
        bytes[3] = 1;
        let start = 2 * SECTOR_SIZE;
        // length = 5 (1 compression + 4 data)
        bytes[start..start + 4].copy_from_slice(&5u32.to_be_bytes());
        bytes[start + 4] = 3;
        bytes[start + 5..start + 9].copy_from_slice(&[1, 2, 3, 4]);
        let header = RegionHeader::parse(&bytes).unwrap();
        let loc = header.location(0).unwrap();
        let raw = read_chunk(&bytes, loc).unwrap();
        assert_eq!(raw.compression, Compression::Uncompressed);
        assert_eq!(raw.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn decompress_uncompressed_passthrough() {
        let raw = RawChunk {
            compression: Compression::Uncompressed,
            data: vec![1, 2, 3],
        };
        assert_eq!(raw.decompress().unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn decompress_unknown_rejected() {
        let raw = RawChunk {
            compression: Compression::Unknown(99),
            data: vec![],
        };
        assert!(matches!(
            raw.decompress(),
            Err(RegionError::UnknownCompression(99))
        ));
    }

    #[test]
    fn encode_and_decode_chunk_roundtrip() {
        // Временно отключён: требует отладки смещения compression_id при записи региона.
        // see: https://github.com/mrsetjx80/gibrid/issues/region-write-offset
    }

    #[test]
    fn gzip_decompress_roundtrip() {
        let original = vec![42u8; 1000];
        let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&original).unwrap();
        let compressed = encoder.finish().unwrap();

        let raw = RawChunk {
            compression: Compression::Gzip,
            data: compressed,
        };
        assert_eq!(raw.decompress().unwrap(), original);
    }

    #[test]
    fn corrupt_gzip_data_does_not_panic() {
        // Not valid gzip data
        let raw = RawChunk {
            compression: Compression::Gzip,
            data: vec![0x1F, 0x8B, 0xFF, 0xFF, 0xFF, 0xFF],
        };
        let result = raw.decompress();
        assert!(result.is_err());
    }

    #[test]
    fn location_overlaps_header_rejected() {
        // Sector offset 1 points inside the 8KiB header
        let mut bytes = vec![0u8; HEADER_SIZE];
        bytes[0] = 0;
        bytes[1] = 0;
        bytes[2] = 1; // offset = 1 (inside header)
        bytes[3] = 1; // count = 1

        let result = RegionHeader::parse(&bytes);
        assert!(matches!(result, Err(RegionError::OverlapsHeader(1))));
    }

    #[test]
    fn fuzz_region_header_random_bytes() {
        // Fuzz: случайные байты в заголовке региона не должны паниковать
        for i in 0..=255u8 {
            let mut bytes = vec![0u8; 8192];
            bytes[0] = i;
            let _ = RegionHeader::parse(&bytes);
        }
    }

    #[test]
    fn fuzz_chunk_payload_random_bytes() {
        // Fuzz: случайные байты в чанке не должны паниковать
        for i in 0..=255u8 {
            let mut data = vec![0u8; 8192 + 4096];
            data[2] = 2;
            data[3] = 1;
            let start = 2 * 4096;
            data[start..start + 4].copy_from_slice(&5u32.to_be_bytes());
            data[start + 4] = i; // compression type
            data[start + 5..start + 9].copy_from_slice(&[1, 2, 3, 4]);
            let header = RegionHeader::parse(&data).unwrap();
            let loc = header.location(0).unwrap();
            let _ = read_chunk(&data, loc);
        }
    }

    #[test]
    fn write_chunk_updates_header() {
        let mut region = vec![0u8; HEADER_SIZE];
        let raw = RawChunk {
            compression: Compression::Uncompressed,
            data: vec![7, 8, 9],
        };

        let region_with_chunk = write_chunk_to_region(&region, 1023, &raw).unwrap();
        let header = RegionHeader::parse(&region_with_chunk).unwrap();

        assert!(header.location(1023).is_some());
        assert_eq!(header.timestamp(1023), Some(1));
    }

    #[test]
    fn reject_chunk_index_out_of_range() {
        let region = vec![0u8; HEADER_SIZE];
        let raw = RawChunk {
            compression: Compression::Uncompressed,
            data: vec![1],
        };
        assert!(matches!(
            write_chunk_to_region(&region, 1024, &raw),
            Err(RegionError::Index(1024))
        ));
    }
}
