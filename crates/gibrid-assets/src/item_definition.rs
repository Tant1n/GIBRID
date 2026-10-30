//! Описание предметов.
//! Предметы — это то, что игрок может держать в инвентаре, крафтить,
// использовать, носить в экипировке.

use gibrid_core::ResourceId;
use std::collections::HashMap;

/// Описание предмета.
#[derive(Debug, Clone)]
pub struct ItemDefinition {
    pub id: ResourceId,
    pub name: String,
    pub max_stack_size: u32,
    pub max_damage: Option<u32>, // None = не изнашивается
    pub is_food: bool,
    pub food_nutrition: Option<u32>, // если is_food
    pub food_saturation: Option<f32>,
    pub can_place_on: Vec<ResourceId>, // какие блоки можно ставить
    pub can_destroy: Vec<ResourceId>, // какие блоки можно ломать
    pub components: ItemComponents, // компоненты предмета (26.3 формат)
}

/// Компоненты предмета (Minecraft 26.3 компонентная система).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemComponents {
    pub custom_name: Option<String>,
    pub custom_model_data: Option<i32>,
    pub enchantments: Vec<(String, i32)>, // enchantment_id, level
    pub attributes: Vec<(String, i64, i32)>, // attribute_id, value, operation
    pub custom_model: Option<String>,
    pub hide_tooltip: bool,
    pub hide_attributes: bool,
}

/// Реестр всех предметов.
#[derive(Debug, Clone, Default)]
pub struct ItemRegistry {
    items: HashMap<ResourceId, ItemDefinition>,
    by_name: HashMap<String, ResourceId>,
}

impl ItemRegistry {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    pub fn register(&mut self, item: ItemDefinition) {
        let id = item.id.clone();
        let name = item.name.clone();
        self.items.insert(id.clone(), item);
        self.by_name.insert(name, id);
    }

    pub fn get(&self, id: &ResourceId) -> Option<&ItemDefinition> {
        self.items.get(id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&ResourceId> {
        self.by_name.get(name)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn all_items(&self) -> impl Iterator<Item = &ItemDefinition> {
        self.items.values()
    }
}

/// Стандартные предметы Minecraft 26.3.
pub mod standard_items {
    use super::*;

    pub fn register_all(registry: &mut ItemRegistry) {
        register_tools(registry);
        register_food(registry);
        register_materials(registry);
        register_redstone_items(registry);
        register_decoration_items(registry);
    }

    fn register_tools(registry: &mut ItemRegistry) {
        let tools: Vec<(&str, &str, u32, Option<u32>, u32, Option<u32>, Option<f32>)> = vec![
            // Мечи
            ("wooden_sword", "Деревянный меч", 1, Some(3), 0, None, None),
            ("stone_sword", "Каменный меч", 2, Some(4), 1, None, None),
            ("iron_sword", "Железный меч", 3, Some(5), 2, None, None),
            ("diamond_sword", "Алмазный меч", 4, Some(6), 3, None, None),
            ("netherite_sword", "Ницеритовый меч", 5, Some(7), 4, None, None),
            // Кирки
            ("wooden_pickaxe", "Деревянная кирка", 1, Some(2), 0, None, None),
            ("stone_pickaxe", "Каменная кирка", 2, Some(3), 1, None, None),
            ("iron_pickaxe", "Железная кирка", 3, Some(4), 2, None, None),
            ("diamond_pickaxe", "Алмазовая кирка", 4, Some(5), 3, None, None),
            ("netherite_pickaxe", "Ницеритовая кирка", 5, Some(6), 4, None, None),
            // Топоры
            ("wooden_axe", "Деревянный топор", 1, Some(2), 0, None, None),
            ("stone_axe", "Каменный топор", 2, Some(3), 1, None, None),
            ("iron_axe", "Железный топор", 3, Some(4), 2, None, None),
            ("diamond_axe", "Алмазный топор", 4, Some(5), 3, None, None),
            ("netherite_axe", "Ницеритовый топор", 5, Some(6), 4, None, None),
            // Лопаты
            ("wooden_shovel", "Деревянная лопата", 1, Some(2), 0, None, None),
            ("stone_shovel", "Каменная лопата", 2, Some(3), 1, None, None),
            ("iron_shovel", "Железная лопата", 3, Some(4), 2, None, None),
            ("diamond_shovel", "Алмазовая лопата", 4, Some(5), 3, None, None),
            ("netherite_shovel", "Ницеретовая лопата", 5, Some(6), 4, None, None),
        ];

        for (id, name, max_stack, max_damage, hardness, food_nutrition, food_saturation) in tools {
            registry.register(ItemDefinition {
                id: ResourceId::minecraft(id).unwrap(),
                name: name.to_string(),
                max_stack_size: max_stack,
                max_damage: max_damage,
                is_food: false,
                food_nutrition: food_nutrition,
                food_saturation: food_saturation.map(|v| v as f32),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                components: ItemComponents::default(),
            });
        }
    }

    fn register_food(registry: &mut ItemRegistry) {
        let foods: Vec<(&str, &str, u32, Option<u32>, bool, u32, f32)> = vec![
            ("apple", "Яблоко", 64, None, true, 4, 2.4),
            ("golden_apple", "Золотое яблоко", 64, None, true, 4, 9.6),
            ("enchanted_golden_apple", "Зачарованное золотое яблоко", 1, None, true, 4, 24.0),
            ("bread", "Хлеб", 64, None, true, 5, 6.0),
            ("cooked_beef", "Жареная говядина", 64, None, true, 8, 12.8),
            ("cooked_porkchop", "Жареная свинина", 64, None, true, 8, 12.8),
            ("cooked_chicken", "Жареная курица", 64, None, true, 6, 7.2),
            ("cooked_mutton", "Жареная баранина", 64, None, true, 6, 9.6),
            ("cooked_rabbit", "Жареная крольчатина", 64, None, true, 5, 6.0),
            ("cooked_cod", "Жареная треска", 64, None, true, 5, 6.0),
            ("cooked_salmon", "Жареная лосось", 64, None, true, 6, 9.6),
            ("cooked_rabbit_stew", "Жареная крольчатина в горшочке", 1, None, true, 10, 18.0),
            ("mushroom_stew", "Грибной суп", 1, None, true, 6, 7.2),
            ("beetroot_soup", "Свекольный суп", 1, None, true, 6, 7.2),
            ("rabbit_stew", "Крольчатина в горшочке", 1, None, true, 10, 18.0),
            ("suspicious_stew", "Тайный суп", 1, None, true, 6, 7.2),
            ("sweet_berries", "Ягоды сладкой ягоды", 64, None, true, 2, 0.4),
            ("glowberries", "Светящиеся ягоды", 64, None, true, 2, 0.4),
            ("dried_kelp", "Высушенная водоросль", 64, None, true, 1, 0.4),
            ("golden_carrot", "Золотая морковь", 64, None, true, 6, 14.4),
            ("golden_apple", "Золотое яблоко", 64, None, true, 4, 9.6),
            ("enchanted_golden_apple", "Зачарованное золотое яблоко", 1, None, true, 4, 24.0),
            ("suspicious_stew", "Тайный суп", 1, None, true, 6, 7.2),
        ];

        for (id, name, max_stack, max_damage, is_food, food_nutrition, food_saturation) in foods {
            registry.register(ItemDefinition {
                id: ResourceId::minecraft(id).unwrap(),
                name: name.to_string(),
                max_stack_size: max_stack,
                max_damage: max_damage,
                is_food: is_food,
                food_nutrition: Some(food_nutrition),
                food_saturation: Some(food_saturation),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                components: ItemComponents::default(),
            });
        }
    }

    fn register_materials(registry: &mut ItemRegistry) {
        let materials: Vec<(&str, &str, u32, Option<u32>, bool, Option<u32>, Option<f32>)> = vec![
            ("iron_ingot", "Железный слиток", 64, None, false, None, None),
            ("gold_ingot", "Золотой слиток", 64, None, false, None, None),
            ("diamond", "Алмаз", 64, None, false, None, None),
            ("emerald", "Изумруд", 64, None, false, None, None),
            ("netherite_ingot", "Ницеритовый слиток", 64, None, false, None, None),
            ("copper_ingot", "Медный слиток", 64, None, false, None, None),
            ("amethyst_shard", "Осколок аметиста", 64, None, false, None, None),
            ("stick", "Палка", 64, None, false, None, None),
            ("string", "Нить", 64, None, false, None, None),
            ("gunpowder", "Серная порошка", 64, None, false, None, None),
            ("flint", "Огниво", 64, None, false, None, None),
            ("feather", "Перо", 64, None, false, None, None),
            ("leather", "Кожа", 64, None, false, None, None),
            ("rabbit_hide", "Кроличья шкура", 64, None, false, None, None),
            ("rabbit_foot", "Лапка кролика", 64, None, false, None, None),
            ("spider_eye", "Глаз паука", 64, None, false, None, None),
            ("fermented_spider_eye", "Броженный глаз паука", 64, None, false, None, None),
            ("ghast_tear", "Слез гаста", 64, None, false, None, None),
            ("blaze_powder", "Порошок пламени", 64, None, false, None, None),
            ("blaze_rod", "Пламеневый прут", 64, None, false, None, None),
            ("magma_cream", "Мазь магмы", 64, None, false, None, None),
            ("glowstone_dust", "Порошок светокамня", 64, None, false, None, None),
            ("redstone", "Редстоун", 64, None, false, None, None),
            ("glowstone_dust", "Порошок светокамня", 64, None, false, None, None),
            ("netherite_ingot", "Ницеритовый слиток", 64, None, false, None, None),
            ("netherite_scrap", "Обломок ницерита", 64, None, false, None, None),
            ("ancient_debris", "Древний обломок", 64, None, false, None, None),
        ];

        for (id, name, max_stack, max_damage, is_food, food_nutrition, food_saturation) in materials {
            registry.register(ItemDefinition {
                id: ResourceId::minecraft(id).unwrap(),
                name: name.to_string(),
                max_stack_size: max_stack,
                max_damage: max_damage,
                is_food: is_food,
                food_nutrition: food_nutrition,
                food_saturation: food_saturation.map(|v| v as f32),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                components: ItemComponents::default(),
            });
        }
    }

    fn register_redstone_items(registry: &mut ItemRegistry) {
        let redstone: Vec<(&str, &str, u32, Option<u32>, bool, Option<u32>, Option<f32>)> = vec![
            ("redstone", "Редстоун", 64, None, false, None, None),
            ("redstone_block", "Блок редстоуна", 64, None, false, None, None),
            ("redstone_torch", "Факел редстоуна", 64, None, false, None, None),
            ("redstone_lamp", "Лампа редстоуна", 64, None, false, None, None),
            ("redstone_block", "Блок редстоуна", 64, None, false, None, None),
            ("observer", "Наблюдатель", 64, None, false, None, None),
            ("repeater", "Повторитель", 64, None, false, None, None),
            ("comparator", "Компаратор", 64, None, false, None, None),
            ("hopper", "Воронка", 64, None, false, None, None),
            ("dropped", "Капельница", 64, None, false, None, None),
            ("dispenser", "Диспенсер", 64, None, false, None, None),
            ("piston", "Поршень", 64, None, false, None, None),
            ("sticky_piston", "Липкий поршень", 64, None, false, None, None),
            ("observer", "Наблюдатель", 64, None, false, None, None),
        ];

        for (id, name, max_stack, max_damage, is_food, food_nutrition, food_saturation) in redstone {
            registry.register(ItemDefinition {
                id: ResourceId::minecraft(id).unwrap(),
                name: name.to_string(),
                max_stack_size: max_stack,
                max_damage: max_damage,
                is_food: is_food,
                food_nutrition: food_nutrition,
                food_saturation: food_saturation.map(|v| v as f32),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                components: ItemComponents::default(),
            });
        }
    }

    fn register_decoration_items(registry: &mut ItemRegistry) {
        let decorations: Vec<(&str, &str, u32, Option<u32>, bool, Option<u32>, Option<f32>)> = vec![
            ("flower_pot", "Цветочный горшок", 64, None, false, None, None),
            ("painting", "Картина", 1, None, false, None, None),
            ("item_frame", "Рамка для предмета", 1, None, false, None, None),
            ("item_frame", "Рамка для предмета", 1, None, false, None, None),
            ("shield", "Щит", 1, Some(336), false, None, None),
            ("elytra", "Крылья крылатки", 1, Some(432), false, None, None),
        ];

        for (id, name, max_stack, max_damage, is_food, food_nutrition, food_saturation) in decorations {
            registry.register(ItemDefinition {
                id: ResourceId::minecraft(id).unwrap(),
                name: name.to_string(),
                max_stack_size: max_stack,
                max_damage: max_damage,
                is_food: is_food,
                food_nutrition: food_nutrition,
                food_saturation: food_saturation.map(|v| v as f32),
                can_place_on: Vec::new(),
                can_destroy: Vec::new(),
                components: ItemComponents::default(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_basic_operations() {
        let mut registry = ItemRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        let item = ItemDefinition {
            id: ResourceId::minecraft("test_item").unwrap(),
            name: "Test Item".to_string(),
            max_stack_size: 64,
            max_damage: None,
            is_food: false,
            food_nutrition: None,
            food_saturation: None,
            can_place_on: Vec::new(),
            can_destroy: Vec::new(),
            components: ItemComponents::default(),
        };

        registry.register(item.clone());
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let retrieved = registry.get(&ResourceId::minecraft("test_item").unwrap());
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Item");
    }

    #[test]
    fn standard_items_register_all() {
        let mut registry = ItemRegistry::new();
        standard_items::register_all(&mut registry);

        assert!(registry.len() > 0);
        assert!(registry.get(&ResourceId::minecraft("iron_sword").unwrap()).is_some());
        assert!(registry.get(&ResourceId::minecraft("apple").unwrap()).is_some());
        assert!(registry.get(&ResourceId::minecraft("iron_ingot").unwrap()).is_some());
        assert!(registry.get(&ResourceId::minecraft("diamond_sword").unwrap()).is_some());
        assert!(registry.get(&ResourceId::minecraft("golden_apple").unwrap()).is_some());
    }

    #[test]
    fn item_components_default() {
        let components = ItemComponents::default();
        assert!(components.enchantments.is_empty());
        assert!(!components.hide_tooltip);
        assert!(!components.hide_attributes);
    }
}
