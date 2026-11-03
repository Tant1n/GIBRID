// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.
use thiserror::Error;

const TAG_END: u8 = 0;
const TAG_BYTE: u8 = 1;
const TAG_SHORT: u8 = 2;
const TAG_INT: u8 = 3;
const TAG_LONG: u8 = 4;
const TAG_FLOAT: u8 = 5;
const TAG_DOUBLE: u8 = 6;
const TAG_BYTE_ARRAY: u8 = 7;
const TAG_STRING: u8 = 8;
const TAG_LIST: u8 = 9;
const TAG_COMPOUND: u8 = 10;
const TAG_INT_ARRAY: u8 = 11;
const TAG_LONG_ARRAY: u8 = 12;

#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List { element_type: u8, items: Vec<Tag> },
    Compound(Vec<(String, Tag)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Tag {
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Tag::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Tag> {
        match self {
            Tag::Compound(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_depth: u32,
    pub max_string_bytes: u32,
    pub max_array_len: u32,
    pub max_list_len: u32,
    pub max_compound_entries: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_string_bytes: 65_535,
            max_array_len: 1_048_576,
            max_list_len: 1_048_576,
            max_compound_entries: 65_536,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NbtError {
    #[error("unexpected end of NBT data at offset {0}")]
    Truncated(usize),
    #[error("unknown NBT tag type {tag} at offset {offset}")]
    UnknownTag { tag: u8, offset: usize },
    #[error("NBT nesting exceeds limit {0}")]
    Depth(u32),
    #[error("NBT string length {len} exceeds remaining input or limit at offset {offset}")]
    StringLen { len: u32, offset: usize },
    #[error("NBT array/list length {len} exceeds remaining input or limit at offset {offset}")]
    Count { len: i32, offset: usize },
    #[error("NBT compound exceeds entry limit at offset {0}")]
    Compound(usize),
    #[error("NBT string is not valid UTF-8 at offset {0}")]
    Utf8(usize),
    #[error("root NBT tag must be a compound")]
    RootNotCompound,
    #[error("trailing bytes after NBT root ({0} unused)")]
    Trailing(usize),
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    depth: u32,
    limits: Limits,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], NbtError> {
        if self.remaining() < n {
            return Err(NbtError::Truncated(self.pos));
        }
        let start = self.pos;
        self.pos += n;
        Ok(&self.data[start..self.pos])
    }

    fn u8(&mut self) -> Result<u8, NbtError> {
        Ok(self.take(1)?[0])
    }

    fn i8(&mut self) -> Result<i8, NbtError> {
        Ok(self.u8()? as i8)
    }

    fn u16(&mut self) -> Result<u16, NbtError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn i16(&mut self) -> Result<i16, NbtError> {
        Ok(self.u16()? as i16)
    }

    fn i32(&mut self) -> Result<i32, NbtError> {
        let b = self.take(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i64(&mut self) -> Result<i64, NbtError> {
        let b = self.take(8)?;
        Ok(i64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn f32(&mut self) -> Result<f32, NbtError> {
        Ok(f32::from_bits(self.i32()? as u32))
    }

    fn f64(&mut self) -> Result<f64, NbtError> {
        Ok(f64::from_bits(self.i64()? as u64))
    }

    fn enter(&mut self) -> Result<(), NbtError> {
        self.depth += 1;
        if self.depth > self.limits.max_depth {
            return Err(NbtError::Depth(self.limits.max_depth));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn checked_count(&self, len: i32, elem_size: usize) -> Result<usize, NbtError> {
        if len < 0 {
            return Err(NbtError::Count {
                len,
                offset: self.pos,
            });
        }
        let len_u = len as u32;
        if len_u > self.limits.max_array_len {
            return Err(NbtError::Count {
                len,
                offset: self.pos,
            });
        }
        let needed = (len as usize)
            .checked_mul(elem_size)
            .ok_or(NbtError::Count {
                len,
                offset: self.pos,
            })?;
        if needed > self.remaining() {
            return Err(NbtError::Count {
                len,
                offset: self.pos,
            });
        }
        Ok(len as usize)
    }

    fn string(&mut self) -> Result<String, NbtError> {
        let len = u32::from(self.u16()?);
        if len > self.limits.max_string_bytes || (len as usize) > self.remaining() {
            return Err(NbtError::StringLen {
                len,
                offset: self.pos,
            });
        }
        let bytes = self.take(len as usize)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| NbtError::Utf8(self.pos - bytes.len()))
    }

    fn payload(&mut self, tag: u8) -> Result<Tag, NbtError> {
        match tag {
            TAG_BYTE => Ok(Tag::Byte(self.i8()?)),
            TAG_SHORT => Ok(Tag::Short(self.i16()?)),
            TAG_INT => Ok(Tag::Int(self.i32()?)),
            TAG_LONG => Ok(Tag::Long(self.i64()?)),
            TAG_FLOAT => Ok(Tag::Float(self.f32()?)),
            TAG_DOUBLE => Ok(Tag::Double(self.f64()?)),
            TAG_BYTE_ARRAY => {
                let len = self.i32()?;
                let n = self.checked_count(len, 1)?;
                let bytes = self.take(n)?;
                Ok(Tag::ByteArray(bytes.iter().map(|b| *b as i8).collect()))
            }
            TAG_STRING => Ok(Tag::String(self.string()?)),
            TAG_LIST => self.list(),
            TAG_COMPOUND => self.compound(),
            TAG_INT_ARRAY => {
                let len = self.i32()?;
                let n = self.checked_count(len, 4)?;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(self.i32()?);
                }
                Ok(Tag::IntArray(items))
            }
            TAG_LONG_ARRAY => {
                let len = self.i32()?;
                let n = self.checked_count(len, 8)?;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(self.i64()?);
                }
                Ok(Tag::LongArray(items))
            }
            TAG_END => Err(NbtError::UnknownTag {
                tag,
                offset: self.pos.saturating_sub(1),
            }),
            other => Err(NbtError::UnknownTag {
                tag: other,
                offset: self.pos.saturating_sub(1),
            }),
        }
    }

    fn list(&mut self) -> Result<Tag, NbtError> {
        self.enter()?;
        let element_type = self.u8()?;
        let len = self.i32()?;
        if len < 0 || (len as u32) > self.limits.max_list_len {
            self.leave();
            return Err(NbtError::Count {
                len,
                offset: self.pos,
            });
        }
        if element_type == TAG_END {
            self.leave();
            if len != 0 {
                return Err(NbtError::Count {
                    len,
                    offset: self.pos,
                });
            }
            return Ok(Tag::List {
                element_type,
                items: Vec::new(),
            });
        }
        let mut items = Vec::new();
        for _ in 0..len {
            items.push(self.payload(element_type)?);
        }
        self.leave();
        Ok(Tag::List {
            element_type,
            items,
        })
    }

    fn compound(&mut self) -> Result<Tag, NbtError> {
        self.enter()?;
        let mut entries = Vec::new();
        loop {
            let tag = self.u8()?;
            if tag == TAG_END {
                break;
            }
            if entries.len() as u32 >= self.limits.max_compound_entries {
                self.leave();
                return Err(NbtError::Compound(self.pos));
            }
            let name = self.string()?;
            let value = self.payload(tag)?;
            entries.push((name, value));
        }
        self.leave();
        Ok(Tag::Compound(entries))
    }
}

/// Parse a named root tag. Returns name, tag, and bytes consumed.
pub fn parse_root(bytes: &[u8], limits: &Limits) -> Result<(String, Tag, usize), NbtError> {
    if bytes.is_empty() {
        return Err(NbtError::Truncated(0));
    }
    let mut reader = Reader {
        data: bytes,
        pos: 0,
        depth: 0,
        limits: *limits,
    };
    let tag = reader.u8()?;
    if tag != TAG_COMPOUND {
        return Err(NbtError::RootNotCompound);
    }
    let name = reader.string()?;
    let value = reader.compound()?;
    Ok((name, value, reader.pos))
}

/// Кодирует именованный корневой тег в NBT байты.
/// Возвращает Vec с полным NBT данными (TYPE + NAME + PAYLOAD + END).
/// Корневой тег всегда Compound, поэтому тип записывается один раз,
/// а write_tag_payload записывает только значение без повторного типа.
pub fn encode_root(name: &str, tag: &Tag) -> Result<Vec<u8>, NbtError> {
    let mut out = Vec::new();
    out.push(TAG_COMPOUND);
    write_string(&mut out, name)?;
    write_tag_payload(&mut out, tag)?;
    out.push(TAG_END);
    Ok(out)
}

/// Пишет тип тега (1 байт) без имени и без значения.
fn write_tag_type(out: &mut Vec<u8>, tag: &Tag) -> Result<(), NbtError> {
    match tag {
        Tag::Byte(_) => out.push(TAG_BYTE),
        Tag::Short(_) => out.push(TAG_SHORT),
        Tag::Int(_) => out.push(TAG_INT),
        Tag::Long(_) => out.push(TAG_LONG),
        Tag::Float(_) => out.push(TAG_FLOAT),
        Tag::Double(_) => out.push(TAG_DOUBLE),
        Tag::ByteArray(_) => out.push(TAG_BYTE_ARRAY),
        Tag::String(_) => out.push(TAG_STRING),
        Tag::List { .. } => out.push(TAG_LIST),
        Tag::Compound(_) => out.push(TAG_COMPOUND),
        Tag::IntArray(_) => out.push(TAG_INT_ARRAY),
        Tag::LongArray(_) => out.push(TAG_LONG_ARRAY),
    }
    Ok(())
}

/// Пишет значение тега без типа и без имени.
fn write_tag_payload(out: &mut Vec<u8>, tag: &Tag) -> Result<(), NbtError> {
    match tag {
        Tag::Byte(v) => out.push(*v as u8),
        Tag::Short(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Int(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Long(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Float(v) => out.extend_from_slice(&v.to_bits().to_be_bytes()),
        Tag::Double(v) => out.extend_from_slice(&v.to_bits().to_be_bytes()),
        Tag::ByteArray(v) => {
            out.extend_from_slice(&(v.len() as i32).to_be_bytes());
            out.extend(v.iter().map(|b| *b as u8));
        }
        Tag::String(s) => write_string(out, s)?,
        Tag::List {
            element_type,
            items,
        } => {
            out.push(*element_type);
            out.extend_from_slice(&(items.len() as i32).to_be_bytes());
            for item in items {
                write_tag_payload(out, item)?;
            }
        }
        Tag::Compound(entries) => {
            for (key, value) in entries {
                write_tag_type(out, value)?;
                write_string(out, key)?;
                write_tag_payload(out, value)?;
            }
            out.push(TAG_END);
        }
        Tag::IntArray(v) => {
            out.extend_from_slice(&(v.len() as i32).to_be_bytes());
            for item in v {
                out.extend_from_slice(&item.to_be_bytes());
            }
        }
        Tag::LongArray(v) => {
            out.extend_from_slice(&(v.len() as i32).to_be_bytes());
            for item in v {
                out.extend_from_slice(&item.to_be_bytes());
            }
        }
    }
    Ok(())
}

fn write_string(out: &mut Vec<u8>, s: &str) -> Result<(), NbtError> {
    let len = s.len() as u16;
    if len as u32 > Limits::default().max_string_bytes {
        return Err(NbtError::StringLen {
            len: len as u32,
            offset: out.len(),
        });
    }
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(s.as_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unnamed_compound_with_int() {
        let bytes = [0x0A, 0, 0, 0x03, 0, 1, b'a', 0, 0, 0, 5, 0x00];
        let (name, tag, used) = parse_root(&bytes, &Limits::default()).unwrap();
        assert_eq!(name, "");
        assert_eq!(tag.get("a").and_then(Tag::as_int), Some(5));
        assert_eq!(used, bytes.len());
    }

    #[test]
    fn rejects_huge_claimed_array() {
        let evil = [0x0A, 0, 0, 0x07, 0, 1, b'a', 0x7F, 0xFF, 0xFF, 0xFF, 0x00];
        assert!(parse_root(&evil, &Limits::default()).is_err());
    }

    #[test]
    fn rejects_truncated_input() {
        assert!(matches!(
            parse_root(&[0x0A, 0], &Limits::default()),
            Err(NbtError::Truncated(_) | NbtError::StringLen { .. })
        ));
    }

    #[test]
    fn depth_limit() {
        let mut limits = Limits::default();
        limits = Limits {
            max_depth: 1,
            ..limits
        };
        // compound containing another compound
        let bytes = [0x0A, 0, 0, 0x0A, 0, 1, b'n', 0x00, 0x00];
        assert!(matches!(
            parse_root(&bytes, &limits),
            Err(NbtError::Depth(1))
        ));
    }

    #[test]
    fn round_trip_named_compound() {
        let original = Tag::Compound(vec![
            ("name".to_string(), Tag::String("test".to_string())),
            ("count".to_string(), Tag::Int(42)),
            ("ratio".to_string(), Tag::Float(std::f32::consts::PI)),
            (
                "nested".to_string(),
                Tag::Compound(vec![("flag".to_string(), Tag::Byte(1))]),
            ),
            (
                "items".to_string(),
                Tag::List {
                    element_type: TAG_INT,
                    items: vec![Tag::Int(1), Tag::Int(2), Tag::Int(3)],
                },
            ),
        ]);

        let encoded = encode_root("root", &original).unwrap();
        let (decoded_name, decoded_tag, _used) = parse_root(&encoded, &Limits::default()).unwrap();

        assert_eq!(decoded_name, "root");
        assert_eq!(decoded_tag, original);
    }

    #[test]
    fn round_trip_byte_array() {
        let original = Tag::Compound(vec![(
            "data".to_string(),
            Tag::ByteArray(vec![1, 2, 3, 4, 5]),
        )]);
        let encoded = encode_root("root", &original).unwrap();
        let (_name, decoded_tag, _used) = parse_root(&encoded, &Limits::default()).unwrap();
        assert_eq!(decoded_tag, original);
    }

    #[test]
    fn round_trip_string_array() {
        let original = Tag::Compound(vec![(
            "names".to_string(),
            Tag::List {
                element_type: TAG_STRING,
                items: vec![
                    Tag::String("alpha".to_string()),
                    Tag::String("beta".to_string()),
                ],
            },
        )]);
        let encoded = encode_root("root", &original).unwrap();
        let (_name, decoded_tag, _used) = parse_root(&encoded, &Limits::default()).unwrap();
        assert_eq!(decoded_tag, original);
    }

    #[test]
    fn fuzz_all_ff_does_not_panic() {
        let bytes = [0xFFu8; 64];
        let result = parse_root(&bytes, &Limits::default());
        assert!(result.is_err());
    }

    #[test]
    fn fuzz_truncated_input_does_not_panic() {
        // Valid start but cut off mid-tag
        let valid = encode_root(
            "root",
            &Tag::Compound(vec![(
                "data".to_string(),
                Tag::String("hello world".to_string()),
            )]),
        )
        .unwrap();
        for cut in 0..valid.len() {
            let truncated = &valid[..cut];
            let result = parse_root(truncated, &Limits::default());
            assert!(
                result.is_err() || result.unwrap().2 <= cut,
                "truncation at {cut} should error or consume fewer bytes"
            );
        }
    }

    #[test]
    fn fuzz_unknown_tag_type_rejected() {
        // TAG_COMPOUND root, then unknown tag type 99
        let mut bytes = Vec::new();
        bytes.push(0x0A); // COMPOUND
        bytes.push(0);
        bytes.push(0); // empty name
        bytes.push(99); // unknown tag
        bytes.push(0); // TAG_END
        let result = parse_root(&bytes, &Limits::default());
        assert!(result.is_err());
    }

    #[test]
    fn fuzz_deeply_nested_compound_hits_depth_limit() {
        let mut limits = Limits::default();
        limits = Limits {
            max_depth: 3,
            ..limits
        };
        // 4 nested compounds should exceed depth 3
        let mut bytes = Vec::new();
        for _ in 0..4 {
            bytes.push(0x0A); // COMPOUND
            bytes.push(0);
            bytes.push(0); // empty name
        }
        bytes.push(0); // END
        bytes.push(0); // END
        bytes.push(0); // END
        bytes.push(0); // END
        let result = parse_root(&bytes, &limits);
        assert!(matches!(result, Err(NbtError::Depth(3))));
    }

    #[test]
    fn fuzz_negative_array_length_rejected() {
        // TAG_INT_ARRAY with length -1 (0xFFFFFFFF)
        let mut bytes = Vec::new();
        bytes.push(0x0A); // COMPOUND
        bytes.push(0);
        bytes.push(1);
        bytes.push(b'a'); // name "a"
        bytes.push(0x0B); // INT_ARRAY
        bytes.extend_from_slice(&(-1i32).to_be_bytes());
        bytes.push(0); // END
        let result = parse_root(&bytes, &Limits::default());
        assert!(result.is_err());
    }
}

