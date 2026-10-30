//! VarInt/VarLong codecs for Minecraft protocol

use byteorder::{ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// VarInt - variable-length integer encoding (1-5 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarInt(pub i32);

impl VarInt {
    pub fn new(value: i32) -> Self { Self(value) }
    pub fn value(&self) -> i32 { self.0 }

    pub fn read(reader: &mut impl Read) -> std::io::Result<Self> {
        let mut result: i32 = 0;
        let mut shift: i32 = 0;
        for _ in 0..5 {
            let byte = reader.read_u8()?;
            result |= ((byte & 0x7F) as i32) << shift;
            if (byte & 0x80) == 0 { break; }
            shift += 7;
        }
        Ok(Self(result))
    }

    pub fn write(&self, writer: &mut impl Write) -> std::io::Result<()> {
        let mut value = self.0 as u32;
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 { byte |= 0x80; }
            writer.write_u8(byte)?;
            if value == 0 { break; }
        }
        Ok(())
    }

    pub fn size(&self) -> usize {
        let mut value = self.0 as u32;
        let mut size = 1;
        while value != 0 { value >>= 7; size += 1; }
        size.min(5)
    }
}

/// VarLong - variable-length long encoding (1-10 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarLong(pub i64);

impl VarLong {
    pub fn new(value: i64) -> Self { Self(value) }
    pub fn value(&self) -> i64 { self.0 }

    pub fn read(reader: &mut impl Read) -> std::io::Result<Self> {
        let mut result: i64 = 0;
        let mut shift: i32 = 0;
        for _ in 0..10 {
            let byte = reader.read_u8()?;
            result |= (byte as i64 & 0x7F) << shift;
            if (byte & 0x80) == 0 { break; }
            shift += 7;
        }
        Ok(Self(result))
    }

    pub fn write(&self, writer: &mut impl Write) -> std::io::Result<()> {
        let mut value = self.0 as u64;
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 { byte |= 0x80; }
            writer.write_u8(byte)?;
            if value == 0 { break; }
        }
        Ok(())
    }

    pub fn size(&self) -> usize {
        let mut value = self.0 as u64;
        let mut size = 1;
        while value != 0 { value >>= 7; size += 1; }
        size.min(10)
    }
}

/// VarIntString - string with VarInt length prefix
#[derive(Debug, Clone)]
pub struct VarIntString(pub String);

impl VarIntString {
    pub fn new(s: String) -> Self { Self(s) }
    pub fn value(&self) -> &str { &self.0 }

    pub fn read(reader: &mut impl Read) -> std::io::Result<Self> {
        let len = VarInt::read(reader)?.value() as usize;
        let mut bytes = vec![0u8; len];
        reader.read_exact(&mut bytes)?;
        let s = String::from_utf8(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Self(s))
    }

    pub fn write(&self, writer: &mut impl Write) -> std::io::Result<()> {
        VarInt::new(self.0.len() as i32).write(writer)?;
        writer.write_all(self.0.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_roundtrip() {
        let values = vec![0, 1, 127, 128, 255, 1000, -1, i32::MAX, i32::MIN];
        for v in values {
            let v = VarInt::new(v);
            let mut buf = Vec::new();
            v.write(&mut buf).unwrap();
            let mut cursor = std::io::Cursor::new(&buf);
            let d = VarInt::read(&mut cursor).unwrap();
            assert_eq!(v.value(), d.value());
        }
    }

    #[test]
    fn varint_string_roundtrip() {
        let s = VarIntString::new("hello world".to_string());
        let mut buf = Vec::new();
        s.write(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(&buf);
        let d = VarIntString::read(&mut cursor).unwrap();
        assert_eq!(s.value(), d.value());
    }
}
