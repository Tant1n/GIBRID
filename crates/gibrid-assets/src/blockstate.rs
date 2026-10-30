//! Парсинг blockstates JSON (сопоставление состояний блоков с моделями).
//!
//! Формат blockstates (Minecraft):
//! {
//!   "variants": {
//!     "": { "model": "cube_all" },
//!     "facing=north": { "model": "cube_all", "y": 90 }
//!   }
//! }

use serde::Deserialize;
use std::collections::HashMap;

/// Одно состояние блока в блокстейте.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Variant {
    pub model: String,
    #[serde(default)]
    pub x: u32,
    #[serde(default)]
    pub y: u32,
    #[serde(default)]
    pub z: u32,
    #[serde(default)]
    pub uvlock: bool,
}

/// Парсированный blockstate файл.
#[derive(Debug, Clone, Deserialize)]
pub struct BlockState {
    pub variants: HashMap<String, Variant>,
}

/// Загрузить blockstate из JSON.
pub fn parse_blockstate(json: &str) -> Result<BlockState, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_blockstate() {
        let json = r#"{
            "variants": {
                "": { "model": "cube_all" },
                "facing=north": { "model": "cube_all", "y": 90 }
            }
        }"#;

        let bs: BlockState = parse_blockstate(json).unwrap();
        assert_eq!(bs.variants.len(), 2);

        let default = bs.variants.get("").unwrap();
        assert_eq!(default.model, "cube_all");
        assert_eq!(default.y, 0);

        let north = bs.variants.get("facing=north").unwrap();
        assert_eq!(north.model, "cube_all");
        assert_eq!(north.y, 90);
    }

    #[test]
    fn parse_blockstate_with_uvlock() {
        let json = r#"{
            "variants": {
                "": { "model": "cube_all", "uvlock": true }
            }
        }"#;

        let bs: BlockState = parse_blockstate(json).unwrap();
        let variant = bs.variants.get("").unwrap();
        assert!(variant.uvlock);
    }

    #[test]
    fn fuzz_blockstate_invalid_json() {
        // Fuzz: некорректный JSON не должен паниковать
        let invalid_inputs = [
            "",
            "not json",
            "{",
            "}",
            "{invalid}",
            "{\"variants\": {}}",
            "{\"variants\": null}",
            "{\"variants\": \"not object\"}",
        ];
        for input in invalid_inputs {
            let result = parse_blockstate(input);
            // Может быть ошибкой, но не паникой
            assert!(result.is_err() || result.is_ok());
        }
    }

    #[test]
    fn fuzz_blockstate_random_bytes() {
        // Fuzz: случайные байты не должны паниковать
        for i in 0..=255u8 {
            let input = vec![i; 64];
            let input_str = String::from_utf8_lossy(&input);
            let result = parse_blockstate(&input_str);
            assert!(result.is_err() || result.is_ok());
        }
    }
}
