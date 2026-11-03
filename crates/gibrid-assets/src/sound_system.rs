// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.//! Звуковая система GIBRID.
//! Управление звуками игры, музыкой, окружающими звуками и звуками сущностей.

use gibrid_core::ResourceId;
use std::collections::HashMap;

/// Звуковой эффект.
#[derive(Debug, Clone)]
pub struct SoundEvent {
    pub id: ResourceId,
    pub volume: f32,
    pub pitch: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub attenuation: f32,
}

/// Звуковое событие для воспроизведения.
#[derive(Debug, Clone)]
pub struct SoundEventInstance {
    pub sound_id: ResourceId,
    pub position: (f32, f32, f32),
    pub volume: f32,
    pub pitch: f32,
    pub loop_sound: bool,
}

/// Реестр звуковых событий.
#[derive(Debug, Clone, Default)]
pub struct SoundRegistry {
    sounds: HashMap<ResourceId, SoundEvent>,
}

impl SoundRegistry {
    pub fn new() -> Self {
        Self {
            sounds: HashMap::new(),
        }
    }

    pub fn register(&mut self, sound: SoundEvent) {
        self.sounds.insert(sound.id.clone(), sound);
    }

    pub fn get(&self, id: &ResourceId) -> Option<&SoundEvent> {
        self.sounds.get(id)
    }

    pub fn len(&self) -> usize {
        self.sounds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }
}

/// Стандартные звуки Minecraft 26.3.
pub mod standard_sounds {
    use super::*;

    pub fn register_all(registry: &mut SoundRegistry) {
        register_block_sounds(registry);
        register_entity_sounds(registry);
        register_ui_sounds(registry);
        register_environment_sounds(registry);
    }

    fn register_block_sounds(registry: &mut SoundRegistry) {
        let sounds = [
            ("stone", "Камень", 1.0, 1.0),
            ("wood", "Дерево", 1.0, 1.0),
            ("glass", "Стекло", 0.5, 1.0),
            ("grass", "Трава", 0.5, 1.0),
            ("gravel", "Гравий", 0.5, 1.0),
            ("sand", "Песок", 0.5, 1.0),
            ("clay", "Глина", 0.5, 1.0),
            ("snow", "Снег", 0.0, 1.0),
            ("ice", "Лёд", 0.5, 1.0),
            ("wool", "Шерсть", 0.0, 1.0),
            ("slime", "Слизь", 0.0, 1.0),
            ("metal", "Металл", 1.0, 1.0),
            ("anvil", "Наковальня", 1.0, 1.0),
            ("water", "Вода", 0.5, 1.0),
            ("lava", "Лава", 1.0, 1.0),
        ];

        for (id, name, volume, pitch) in sounds {
            registry.register(SoundEvent {
                id: ResourceId::minecraft(id).unwrap(),
                volume,
                pitch,
                min_distance: 0.0,
                max_distance: 16.0,
                attenuation: 1.0,
            });
        }
    }

    fn register_entity_sounds(registry: &mut SoundRegistry) {
        let sounds = [
            ("player_hurt", "Удар игрока", 1.0, 1.0),
            ("player_death", "Смерть игрока", 1.0, 0.0),
            ("cow_hurt", "Удар коровы", 0.5, 1.0),
            ("cow_death", "Смерть коровы", 0.5, 1.0),
            ("pig_hurt", "Удар свиньи", 0.5, 1.0),
            ("pig_death", "Смерть свиньи", 0.5, 1.0),
            ("chicken_hurt", "Удар курицы", 0.5, 1.0),
            ("chicken_death", "Смерть курицы", 0.5, 1.0),
            ("sheep_hurt", "Удар овцы", 0.5, 1.0),
            ("sheep_death", "Смерть овцы", 0.5, 1.0),
            ("zombie_hurt", "Удар зомби", 1.0, 1.0),
            ("zombie_death", "Смерть зомби", 1.0, 1.0),
            ("skeleton_hurt", "Удар скелета", 1.0, 1.0),
            ("skeleton_death", "Смерть скелета", 1.0, 1.0),
            ("creeper_hurt", "Удар крипера", 1.0, 1.0),
            ("creeper_death", "Смерть крипера", 1.0, 1.0),
            ("spider_hurt", "Удар паука", 1.0, 1.0),
            ("spider_death", "Смерть паука", 1.0, 1.0),
        ];

        for (id, name, volume, pitch) in sounds {
            registry.register(SoundEvent {
                id: ResourceId::minecraft(id).unwrap(),
                volume,
                pitch,
                min_distance: 0.0,
                max_distance: 16.0,
                attenuation: 1.0,
            });
        }
    }

    fn register_ui_sounds(registry: &mut SoundRegistry) {
        let ui_sounds = [
            ("ui.button.click", "Кнопка", 1.0, 1.0),
            ("ui.button.toggle", "Переключатель", 1.0, 1.0),
            ("ui.toast.in", "Появление уведомления", 1.0, 1.0),
            ("ui.toast.out", "Исчезновение уведомления", 1.0, 1.0),
            ("ui.inventory.click", "Клик инвентаря", 0.5, 1.0),
            ("ui.inventory.pickup", "Поднятие предмета", 0.5, 1.0),
            ("ui.inventory.drop", "Бросание предмета", 0.5, 1.0),
            ("ui.cart.move", "Движение вагонетки", 1.0, 1.0),
        ];

        for (id, name, volume, pitch) in ui_sounds {
            registry.register(SoundEvent {
                id: ResourceId::minecraft(id).unwrap(),
                volume,
                pitch,
                min_distance: 0.0,
                max_distance: 16.0,
                attenuation: 1.0,
            });
        }
    }

    fn register_environment_sounds(registry: &mut SoundRegistry) {
        let env_sounds = [
            ("ambient.cave", "Пещера", 0.5, 1.0),
            ("ambient.underwater", "Под водой", 0.5, 1.0),
            ("ambient.weather.rain", "Дождь", 0.5, 1.0),
            ("ambient.weather.thunder", "Гром", 1.0, 1.0),
            ("ambient.weather.snow", "Снег", 0.0, 1.0),
            ("weather.rain", "Дождь", 0.5, 1.0),
            ("weather.snow", "Снег", 0.0, 1.0),
            ("biome.desert", "Пустыня", 0.5, 1.0),
            ("biome.swamp", "Болото", 0.5, 1.0),
            ("biome.jungle", "Джунгли", 0.5, 1.0),
        ];

        for (id, name, volume, pitch) in env_sounds {
            registry.register(SoundEvent {
                id: ResourceId::minecraft(id).unwrap(),
                volume,
                pitch,
                min_distance: 0.0,
                max_distance: 32.0,
                attenuation: 1.0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_registry_basic_operations() {
        let mut registry = SoundRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        let sound = SoundEvent {
            id: ResourceId::minecraft("test_sound").unwrap(),
            volume: 1.0,
            pitch: 1.0,
            min_distance: 0.0,
            max_distance: 16.0,
            attenuation: 1.0,
        };

        registry.register(sound.clone());
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let retrieved = registry.get(&ResourceId::minecraft("test_sound").unwrap());
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().volume, 1.0);
    }

    #[test]
    fn standard_sounds_register_all() {
        let mut registry = SoundRegistry::new();
        standard_sounds::register_all(&mut registry);

        assert!(registry.len() > 0);
        assert!(registry.get(&ResourceId::minecraft("stone").unwrap()).is_some());
        assert!(registry.get(&ResourceId::minecraft("water").unwrap()).is_some());
        assert!(registry.get(&ResourceId::minecraft("player_hurt").unwrap()).is_some());
    }
}

