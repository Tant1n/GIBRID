// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.//! Парсинг block model JSON с наследованием (parent).
//!
//! Формат модели (Minecraft):
//! {
//!   "parent": "block/cube_all",
//!   "textures": {
//!     "all": "block/stone"
//!   }
//! }

use serde::Deserialize;
use std::collections::HashMap;

/// Модель блока с текстурами и опциональным родителем.
#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    #[serde(default)]
    pub parent: Option<String>,
    pub textures: HashMap<String, String>,
}

/// Загрузить модель из JSON.
pub fn parse_model(json: &str) -> Result<Model, serde_json::Error> {
    serde_json::from_str(json)
}

/// Разрешить текстуру модели через цепочку parent.
/// Возвращает最終е имя текстуры или None, если не найдено.
pub fn resolve_texture<'a>(
    model: &'a Model,
    all_models: &HashMap<&'a str, &'a Model>,
    texture_name: &str,
) -> Option<&'a str> {
    // Сначала ищем в текстурах самой модели
    if let Some(resolved) = model.textures.get(texture_name) {
        return Some(resolved);
    }

    // Luego идем в parent
    if let Some(parent_name) = &model.parent {
        if let Some(parent_model) = all_models.get(parent_name.as_str()) {
            return resolve_texture(parent_model, all_models, texture_name);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_model() {
        let json = r#"{
            "parent": "block/cube_all",
            "textures": {
                "all": "block/stone"
            }
        }"#;

        let model: Model = parse_model(json).unwrap();
        assert_eq!(model.parent.as_deref(), Some("block/cube_all"));
        assert_eq!(model.textures.get("all"), Some(&"block/stone".to_string()));
    }

    #[test]
    fn resolve_texture_from_model() {
        let json = r#"{
            "textures": {
                "all": "block/stone"
            }
        }"#;

        let model: Model = parse_model(json).unwrap();
        let models: HashMap<&str, &Model> = HashMap::new();
        assert_eq!(
            resolve_texture(&model, &models, "all"),
            Some("block/stone")
        );
        assert_eq!(resolve_texture(&model, &models, "nonexistent"), None);
    }

    #[test]
    fn resolve_texture_via_parent() {
        let parent_json = r#"{
            "textures": {
                "all": "block/stone"
            }
        }"#;
        let child_json = r#"{
            "parent": "block/cube_all",
            "textures": {
                "side": "block/dirt"
            }
        }"#;

        let parent: Model = parse_model(parent_json).unwrap();
        let child: Model = parse_model(child_json).unwrap();

        let mut models: HashMap<&str, &Model> = HashMap::new();
        models.insert("block/cube_all", &parent);
        models.insert("block/dirt", &child);

        assert_eq!(resolve_texture(&child, &models, "all"), Some("block/stone"));
        assert_eq!(resolve_texture(&child, &models, "side"), Some("block/dirt"));
        assert_eq!(resolve_texture(&child, &models, "nonexistent"), None);
    }
}

