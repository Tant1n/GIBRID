//! Чанковые секции: палитры block states, packed индексы.
//! Формат Anvil (MCA): чанк — это NBT с корнем "Level", содержащий список "Sections".

use std::collections::HashMap;
use std::str::FromStr;

use crate::nbt::{parse_root, Limits, NbtError, Tag};
use gibrid_core::ResourceId;
use thiserror::Error;

/// Ошибки при чтении секций чанка.
#[derive(Debug, Error)]
pub enum ChunkError {
    #[error("NBT error: {0}")]
    Nbt(#[from] NbtError),
    #[error("missing NBT field `{0}`")]
    MissingField(String),
    #[error("unexpected NBT tag type for `{0}`")]
    UnexpectedTag(String),
    #[error("palette entry is missing `Name` field")]
    PaletteMissingName,
    #[error("chunk data is not a long array")]
    DataNotLongArray,
}

/// Одно состояние блока в палитре (имя + свойства).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockStateEntry {
    pub name: ResourceId,
    pub properties: Vec<(String, String)>,
}

/// Палитра блок-состояний одной секции чанка.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockStatePalette {
    pub entries: Vec<BlockStateEntry>,
}

/// Секция чанка (16×16×16 блоков).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSection {
    /// Индекс секции по Y (обычно 0..=15 для высоты мира 384).
    pub y: i8,
    pub block_states: BlockStatePalette,
    /// Packed индексы блоков (один на позицию, 4096 позиций в секции).
    pub block_data: Vec<u64>,
    /// Битов на один блок в packed массиве.
    pub bits_per_block: u32,
}

impl ChunkSection {
    /// Линейная позиция блока в секции (0..4095).
    fn position_index(x: usize, y: usize, z: usize) -> Option<usize> {
        if x >= 16 || y >= 16 || z >= 16 {
            return None;
        }
        Some((y * 16 + z) * 16 + x)
    }

    /// Декодировать индекс блок-состояния из packed массива по позиции (x, y, z).
    pub fn block_index_at(&self, x: usize, y: usize, z: usize) -> Option<usize> {
        let pos = Self::position_index(x, y, z)?;
        if self.bits_per_block == 0 {
            // Секция состоит из одного состояния (воздуха), индекс всегда 0.
            return Some(0);
        }
        let bit_start = pos * self.bits_per_block as usize;
        let mut value: u64 = 0;
        for i in 0..self.bits_per_block as usize {
            let bit = bit_start + i;
            let word = bit / 64;
            let offset = bit % 64;
            if word >= self.block_data.len() {
                return None;
            }
            if (self.block_data[word] >> offset) & 1 == 1 {
                value |= 1u64 << i;
            }
        }
        Some(value as usize)
    }

    /// Получить описание блок-состояния по позиции.
    pub fn block_state_at(&self, x: usize, y: usize, z: usize) -> Option<&BlockStateEntry> {
        let idx = self.block_index_at(x, y, z)?;
        self.block_states.entries.get(idx)
    }
}

/// Рассчитать bits-per-block из размера палитры (логика Minecraft).
pub fn bits_per_block_for_palette(palette_len: usize) -> u32 {
    if palette_len <= 1 {
        return 0;
    }
    // Битов достаточно для хранения индекса 0..(palette_len-1)
    let needed = (palette_len - 1).ilog2() + 1;
    needed.clamp(4, 8)
}

/// Вспомогательный доступ к полю compound по имени.
fn compound_get<'a>(tag: &'a Tag, key: &str) -> Option<&'a Tag> {
    match tag {
        Tag::Compound(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

/// Получить элементы списка NBT.
fn list_items(tag: &Tag) -> Option<&Vec<Tag>> {
    match tag {
        Tag::List { items, .. } => Some(items),
        _ => None,
    }
}

/// Получить строку из тега NBT.
fn tag_as_string(tag: &Tag) -> Option<&str> {
    match tag {
        Tag::String(s) => Some(s),
        _ => None,
    }
}

/// Получить long-массив из тега NBT.
fn tag_as_long_array(tag: &Tag) -> Option<&Vec<i64>> {
    match tag {
        Tag::LongArray(v) => Some(v),
        _ => None,
    }
}

/// Распарсить одну секцию чанка из NBT compound.
fn parse_section(compound: &[(String, Tag)], y: i8) -> Result<ChunkSection, ChunkError> {
    let compound_map: HashMap<&str, &Tag> = compound.iter().map(|(k, v)| (k.as_str(), v)).collect();

    let block_states_tag = compound_map
        .get("block_states")
        .ok_or_else(|| ChunkError::MissingField("block_states".to_string()))?;

    let palette_tag = compound_get(block_states_tag, "palette")
        .ok_or_else(|| ChunkError::MissingField("block_states.palette".to_string()))?;

    let palette_items = list_items(palette_tag)
        .ok_or_else(|| ChunkError::UnexpectedTag("block_states.palette".to_string()))?;

    let mut entries = Vec::with_capacity(palette_items.len());
    for (i, entry_tag) in palette_items.iter().enumerate() {
        let entry_compound = match entry_tag {
            Tag::Compound(e) => e,
            _ => return Err(ChunkError::UnexpectedTag(format!("palette[{}]", i))),
        };
        let entry_map: HashMap<&str, &Tag> = entry_compound
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect();

        let name_tag = entry_map
            .get("Name")
            .ok_or(ChunkError::PaletteMissingName)?;
        let name_str = tag_as_string(name_tag)
            .ok_or(ChunkError::UnexpectedTag("palette[].Name".to_string()))?;
        let name = ResourceId::from_str(name_str).map_err(|_| ChunkError::PaletteMissingName)?;

        let properties: Vec<(String, String)> = if let Some(props_tag) = entry_map.get("Properties")
        {
            match props_tag {
                Tag::Compound(prop_entries) => prop_entries
                    .iter()
                    .filter_map(|(k, v)| tag_as_string(v).map(|s| (k.clone(), s.to_string())))
                    .collect(),
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };

        entries.push(BlockStateEntry { name, properties });
    }

    let palette = BlockStatePalette { entries };
    let bits_per_block = bits_per_block_for_palette(palette.entries.len());

    let data_tag = compound_get(block_states_tag, "data")
        .ok_or_else(|| ChunkError::MissingField("block_states.data".to_string()))?;

    let long_array = tag_as_long_array(data_tag).ok_or(ChunkError::DataNotLongArray)?;

    let block_data: Vec<u64> = long_array.iter().map(|v| *v as u64).collect();

    Ok(ChunkSection {
        y,
        block_states: palette,
        block_data,
        bits_per_block,
    })
}

/// Распарсить все секции чанка из байтов NBT чанка (уже распакованных из MCA).
pub fn parse_chunk_sections(nbt_bytes: &[u8]) -> Result<Vec<ChunkSection>, ChunkError> {
    let (_root_name, root_tag, _used) = parse_root(nbt_bytes, &Limits::default())?;

    let level = compound_get(&root_tag, "Level")
        .ok_or_else(|| ChunkError::MissingField("Level".to_string()))?;

    let sections_tag = compound_get(level, "Sections")
        .ok_or_else(|| ChunkError::MissingField("Level.Sections".to_string()))?;

    let sections_items = list_items(sections_tag)
        .ok_or_else(|| ChunkError::UnexpectedTag("Level.Sections".to_string()))?;

    let mut result = Vec::with_capacity(sections_items.len());
    for (i, section_tag) in sections_items.iter().enumerate() {
        let section_compound = match section_tag {
            Tag::Compound(e) => e,
            _ => return Err(ChunkError::UnexpectedTag(format!("Sections[{}]", i))),
        };

        let y_tag = section_compound
            .iter()
            .find(|(k, _)| k == "Y")
            .map(|(_, v)| v)
            .ok_or_else(|| ChunkError::MissingField(format!("Sections[{}].Y", i)))?;
        let y = match y_tag {
            Tag::Int(v) => *v as i8,
            Tag::Byte(v) => *v,
            _ => return Err(ChunkError::UnexpectedTag(format!("Sections[{}].Y", i))),
        };

        let section = parse_section(section_compound, y)?;
        result.push(section);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_per_block_calculation() {
        assert_eq!(bits_per_block_for_palette(0), 0);
        assert_eq!(bits_per_block_for_palette(1), 0);
        assert_eq!(bits_per_block_for_palette(2), 4); // min 4 bits
        assert_eq!(bits_per_block_for_palette(16), 4);
        assert_eq!(bits_per_block_for_palette(17), 5);
        assert_eq!(bits_per_block_for_palette(256), 8);
    }

    #[test]
    fn parse_simple_palette_single_block() {
        // Одна секция Y=0, палитра: 1 элемент (воздух)
        let mut palette_entry = Vec::new();
        palette_entry.push(("Name".to_string(), Tag::String("minecraft:air".to_string())));
        palette_entry.push(("Properties".to_string(), Tag::Compound(Vec::new())));

        let mut sections_entries = Vec::new();
        sections_entries.push(("Y".to_string(), Tag::Int(0)));
        sections_entries.push((
            "block_states".to_string(),
            Tag::Compound(vec![
                (
                    "palette".to_string(),
                    Tag::List {
                        element_type: 10,
                        items: vec![Tag::Compound(palette_entry)],
                    },
                ),
                ("data".to_string(), Tag::LongArray(vec![0; 64])),
            ]),
        ));

        let section = parse_section(&sections_entries, 0).unwrap();
        assert_eq!(section.y, 0);
        assert_eq!(section.block_states.entries.len(), 1);
        assert_eq!(
            section.block_states.entries[0].name.namespace(),
            "minecraft"
        );
        assert_eq!(section.block_states.entries[0].name.path(), "air");
        assert_eq!(section.bits_per_block, 0); // 1 элемент -> 0 бит

        // При позиции (0,0,0) должен быть индекс 0 (воздух)
        assert_eq!(section.block_index_at(0, 0, 0), Some(0));
        assert_eq!(section.block_state_at(0, 0, 0).unwrap().name.path(), "air");
        // Любая другая позиция тоже воздух (0 бит)
        assert_eq!(section.block_index_at(15, 15, 15), Some(0));
    }

    #[test]
    fn parse_palette_two_blocks() {
        // Два элемента палитры: камень (0) и грязь (1)
        let mut palette = Vec::new();
        palette.push((
            "Name".to_string(),
            Tag::String("minecraft:stone".to_string()),
        ));
        palette.push(("Properties".to_string(), Tag::Compound(Vec::new())));
        palette.push((
            "Name".to_string(),
            Tag::String("minecraft:dirt".to_string()),
        ));
        palette.push(("Properties".to_string(), Tag::Compound(Vec::new())));

        let mut sections_entries = Vec::new();
        sections_entries.push(("Y".to_string(), Tag::Int(3)));
        sections_entries.push((
            "block_states".to_string(),
            Tag::Compound(vec![
                (
                    "palette".to_string(),
                    Tag::List {
                        element_type: 10,
                        items: vec![
                            Tag::Compound(palette[0..2].to_vec()),
                            Tag::Compound(palette[2..4].to_vec()),
                        ],
                    },
                ),
                (
                    "data".to_string(),
                    // 4096 блоков * 4 бита = 16384 бит = 256 long, все нули = индекс 0 (камень)
                    Tag::LongArray(vec![0; 256]),
                ),
            ]),
        ));

        let section = parse_section(&sections_entries, 3).unwrap();
        assert_eq!(section.y, 3);
        assert_eq!(section.block_states.entries.len(), 2);
        assert_eq!(section.block_states.entries[0].name.path(), "stone");
        assert_eq!(section.block_states.entries[1].name.path(), "dirt");
        assert_eq!(section.bits_per_block, 4);

        // Все блоки — индекс 0 (камень)
        assert_eq!(section.block_index_at(0, 0, 0), Some(0));
        assert_eq!(section.block_index_at(15, 15, 15), Some(0));
        assert_eq!(
            section.block_state_at(0, 0, 0).unwrap().name.path(),
            "stone"
        );
    }

    #[test]
    fn parse_chunk_sections_from_nbt() {
        // Собираем полный NBT чанка вручную: корневой compound -> "Level" -> Sections
        let mut block_states_entries = Vec::new();
        block_states_entries.push((
            "palette".to_string(),
            Tag::List {
                element_type: 10,
                items: vec![
                    Tag::Compound(vec![
                        (
                            "Name".to_string(),
                            Tag::String("minecraft:stone".to_string()),
                        ),
                        ("Properties".to_string(), Tag::Compound(Vec::new())),
                    ]),
                    Tag::Compound(vec![
                        (
                            "Name".to_string(),
                            Tag::String("minecraft:dirt".to_string()),
                        ),
                        ("Properties".to_string(), Tag::Compound(Vec::new())),
                    ]),
                ],
            },
        ));
        block_states_entries.push(("data".to_string(), Tag::LongArray(vec![0; 256])));

        let mut section_entries = Vec::new();
        section_entries.push(("Y".to_string(), Tag::Int(0)));
        section_entries.push((
            "block_states".to_string(),
            Tag::Compound(block_states_entries),
        ));

        let mut sections_items = Vec::new();
        sections_items.push(Tag::Compound(section_entries.clone()));

        let mut level_entries = Vec::new();
        level_entries.push((
            "Sections".to_string(),
            Tag::List {
                element_type: 10,
                items: sections_items,
            },
        ));
        level_entries.push(("xPos".to_string(), Tag::Int(5)));
        level_entries.push(("zPos".to_string(), Tag::Int(-3)));
        level_entries.push(("DataVersion".to_string(), Tag::Int(5023)));

        // Проверяем логику parse_section на уже распакованной структуре.
        let section = parse_section(&section_entries, 0).unwrap();
        assert_eq!(section.y, 0);
        assert_eq!(section.block_states.entries.len(), 2);
        assert_eq!(section.block_states.entries[0].name.path(), "stone");
        assert_eq!(section.block_states.entries[1].name.path(), "dirt");
        assert_eq!(section.bits_per_block, 4);
    }
}
