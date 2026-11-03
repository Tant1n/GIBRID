// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.


// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.//! Описание блока и его состояний.
//! Каждый блок имеет идентификатор, имя, набор состояний (block states) и ссылку на модель.
//! Это основа для полноценного переноса всех блоков Minecraft 26.3.

use serde::Deserialize;
use std::collections::HashMap;
use gibrid_core::ResourceId;

/// Состояние блока (block state) — комбинация свойств.
/// Например: facing=north, powered=true, waterlogged=false
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BlockState {
    pub properties: HashMap<String, String>,
}

/// Описание блока (block definition).
#[derive(Debug, Clone)]
pub struct BlockDefinition {
    pub id: ResourceId,
    pub name: String,
    pub hardness: f32,
    pub blast_resistance: f32,
    pub is_opaque: bool,
    pub is_full_cube: bool,
    pub is_transparent: bool,
    pub emits_light: u8, // 0-15
    pub block_states: Vec<BlockState>,
    pub default_state: BlockState,
}

/// Реестр всех блоков игры.
#[derive(Debug, Clone, Default)]
pub struct BlockRegistry {
    blocks: HashMap<ResourceId, BlockDefinition>,
    by_name: HashMap<String, ResourceId>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    pub fn register(&mut self, block: BlockDefinition) {
        let id = block.id.clone();
        let name = block.name.clone();
        self.blocks.insert(id.clone(), block);
        self.by_name.insert(name, id);
    }

    pub fn get(&self, id: &ResourceId) -> Option<&BlockDefinition> {
        self.blocks.get(id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&ResourceId> {
        self.by_name.get(name)
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn all_blocks(&self) -> impl Iterator<Item = &BlockDefinition> {
        self.blocks.values()
    }
}

/// Стандартные блоки Minecraft 26.3.
/// Каждый блок_register вызывается при инициализации реестра.
pub mod standard_blocks {
    use super::*;

    pub fn register_all(registry: &mut BlockRegistry) {
        register_full_blocks(registry);
        register_fluids(registry);
        register_redstone(registry);
        register_decorations(registry);
        register_natural(registry);
    }

    fn register_full_blocks(registry: &mut BlockRegistry) {
        // Полные кубы (full cubes)
        let full_cubes = [
            // Камень и камни
            ("stone", "Камень", 1.5, 6.0, true, true, false, 0),
            ("deepslate", "Глубинный сланец", 1.5, 6.0, true, true, false, 0),
            ("cobblestone", "Каменный кирпич", 2.0, 6.0, true, true, false, 0),
            ("stone_bricks", "Каменные кирпичи", 2.0, 6.0, true, true, false, 0),
            ("mossy_cobblestone", "Покрытый мхом булыжник", 2.0, 6.0, true, true, false, 0),
            ("mossy_stone_bricks", "Покрытый мхом каменный кирпич", 2.0, 6.0, true, true, false, 0),
            ("cracked_stone_bricks", "Трескающийся каменный кирпич", 2.0, 6.0, true, true, false, 0),
            ("diorite", "Диорит", 1.5, 6.0, true, true, false, 0),
            ("andesite", "Андезит", 1.5, 6.0, true, true, false, 0),
            ("granite", "Гранит", 1.5, 6.0, true, true, false, 0),
            ("tuff", "Туф", 1.5, 6.0, true, true, false, 0),
            ("calcite", "Кальцит", 1.0, 3.0, true, true, false, 0),
            ("deepslate_tiles", "Плитка глубинного сланца", 2.0, 6.0, true, true, false, 0),
            ("deepslate_bricks", "Кирпич глубинного сланца", 2.0, 6.0, true, true, false, 0),
            // Земля и грунт
            ("dirt", "Глина", 0.5, 0.5, true, true, false, 0),
            ("coarse_dirt", "Грубая глина", 0.5, 0.5, true, true, false, 0),
            ("podzol", "Подзол", 0.6, 0.6, true, true, false, 0),
            ("rooted_dirt", "Корневищная глина", 0.6, 0.6, true, true, false, 0),
            ("mud", "Грязь", 0.5, 0.5, true, true, false, 0),
            ("sand", "Песок", 0.5, 0.5, true, true, false, 0),
            ("red_sand", "Красный песок", 0.5, 0.5, true, true, false, 0),
            ("gravel", "Гравий", 0.6, 0.6, true, true, false, 0),
            ("sandstone", "Песчаник", 0.8, 2.0, true, true, false, 0),
            ("chiseled_sandstone", "Резной песчаник", 2.0, 6.0, true, true, false, 0),
            ("cut_sandstone", "Рваный песчаник", 2.0, 6.0, true, true, false, 0),
            ("cut_sandstone_slab", "Плитка из рваного песчаника", 2.0, 6.0, true, true, false, 0),
            ("red_sandstone", "Красный песчаник", 0.8, 2.0, true, true, false, 0),
            ("cut_red_sandstone", "Рваный красный песчаник", 2.0, 6.0, true, true, false, 0),
            ("cut_red_sandstone_slab", "Плитка из рваного красного песчаника", 2.0, 6.0, true, true, false, 0),
            // Руды
            ("coal_ore", "Угольная руда", 3.0, 3.0, true, true, false, 0),
            ("iron_ore", "Железная руда", 3.0, 3.0, true, true, false, 0),
            ("gold_ore", "Золотая руда", 3.0, 3.0, true, true, false, 0),
            ("diamond_ore", "Алмазная руда", 3.0, 5.0, true, true, false, 0),
            ("emerald_ore", "Изумрудная руда", 3.0, 5.0, true, true, false, 0),
            ("lapis_ore", "Лазуритовая руда", 3.0, 3.0, true, true, false, 0),
            ("copper_ore", "Медная руда", 3.0, 3.0, true, true, false, 0),
            ("deepslate_coal_ore", "Угольная руда глубинного сланца", 3.0, 4.5, true, true, false, 0),
            ("deepslate_iron_ore", "Железная руда глубинного сланца", 3.0, 4.5, true, true, false, 0),
            ("deepslate_gold_ore", "Золотая руда глубинного сланца", 3.0, 4.5, true, true, false, 0),
            ("deepslate_diamond_ore", "Алмазная руда глубинного сланца", 3.0, 6.0, true, true, false, 0),
            ("deepslate_emerald_ore", "Изумрудная руда глубинного сланца", 3.0, 6.0, true, true, false, 0),
            ("deepslate_lapis_ore", "Лазуритовая руда глубинного сланца", 3.0, 4.5, true, true, false, 0),
            ("deepslate_copper_ore", "Медная руда глубинного сланца", 3.0, 4.5, true, true, false, 0),
            // Жидкости
            ("water", "Вода", 0.0, 0.0, false, false, true, 0),
            ("lava", "Лава", 0.0, 0.0, false, false, true, 15),
            // Деревья и древесина
            ("oak_log", "Дубовый ствол", 2.0, 3.0, true, true, false, 0),
            ("oak_wood", "Дубовая древесина", 2.0, 3.0, true, true, false, 0),
            ("oak_leaves", "Дубовые листья", 0.2, 0.0, false, false, false, 0),
            ("oak_sapling", "Росток дуба", 0.0, 0.0, false, false, false, 0),
            ("spruce_log", "Еловый ствол", 2.0, 3.0, true, true, false, 0),
            ("spruce_wood", "Еловая древесина", 2.0, 3.0, true, true, false, 0),
            ("spruce_leaves", "Еловые листья", 0.2, 0.0, false, false, false, 0),
            ("spruce_sapling", "Росток ели", 0.0, 0.0, false, false, false, 0),
            ("birch_log", "Берёзовый ствол", 2.0, 3.0, true, true, false, 0),
            ("birch_wood", "Берёзовая древесина", 2.0, 3.0, true, true, false, 0),
            ("birch_leaves", "Берёзовые листья", 0.2, 0.0, false, false, false, 0),
            ("birch_sapling", "Росток берёзы", 0.0, 0.0, false, false, false, 0),
            ("jungle_log", "Тропический ствол", 2.0, 3.0, true, true, false, 0),
            ("jungle_wood", "Тропическая древесина", 2.0, 3.0, true, true, false, 0),
            ("jungle_leaves", "Тропические листья", 0.2, 0.0, false, false, false, 0),
            ("jungle_sapling", "Тропический росток", 0.0, 0.0, false, false, false, 0),
            ("acacia_log", "Акациевый ствол", 2.0, 3.0, true, true, false, 0),
            ("acacia_wood", "Акациевая древесина", 2.0, 3.0, true, true, false, 0),
            ("acacia_leaves", "Акациевые листья", 0.2, 0.0, false, false, false, 0),
            ("acacia_sapling", "Акациевый росток", 0.0, 0.0, false, false, false, 0),
            ("dark_oak_log", "Тёмный дубовый ствол", 2.0, 3.0, true, true, false, 0),
            ("dark_oak_wood", "Тёмная дубовая древесина", 2.0, 3.0, true, true, false, 0),
            ("dark_oak_leaves", "Тёмнодубовые листья", 0.2, 0.0, false, false, false, 0),
            ("dark_oak_sapling", "Тёмнодубовый росток", 0.0, 0.0, false, false, false, 0),
            ("mangrove_log", "Мангровый ствол", 2.0, 3.0, true, true, false, 0),
            ("mangrove_wood", "Мангровая древесина", 2.0, 3.0, true, true, false, 0),
            ("mangrove_leaves", "Мангровые листья", 0.2, 0.0, false, false, false, 0),
            ("mangrove_sapling", "Мангровый росток", 0.0, 0.0, false, false, false, 0),
            ("cherry_log", "Вишнёвый ствол", 2.0, 3.0, true, true, false, 0),
            ("cherry_wood", "Вишнёвая древесина", 2.0, 3.0, true, true, false, 0),
            ("cherry_leaves", "Вишнёвые листья", 0.2, 0.0, false, false, false, 0),
            ("cherry_sapling", "Вишнёвый росток", 0.0, 0.0, false, false, false, 0),
            // Цветы и растения
            ("poppy", "Мако", 0.0, 0.0, false, false, false, 0),
            ("dandelion", "Одуванчик", 0.0, 0.0, false, false, false, 0),
            ("blue_orchid", "Синяя орхидея", 0.0, 0.0, false, false, false, 0),
            ("allium", "Лук", 0.0, 0.0, false, false, false, 0),
            ("azure_bluet", "Азуровый цикорий", 0.0, 0.0, false, false, false, 0),
            ("red_tulip", "Красный тюльпан", 0.0, 0.0, false, false, false, 0),
            ("orange_tulip", "Оранжевый тюльпан", 0.0, 0.0, false, false, false, 0),
            ("white_tulip", "Белый тюльпан", 0.0, 0.0, false, false, false, 0),
            ("pink_tulip", "Розовый тюльпан", 0.0, 0.0, false, false, false, 0),
            ("oxeye_daisy", "Ромашка", 0.0, 0.0, false, false, false, 0),
            ("cornflower", "Василёк", 0.0, 0.0, false, false, false, 0),
            ("lily_of_the_valley", "Ландыш", 0.0, 0.0, false, false, false, 0),
            ("wither_rose", "Клык Визера", 0.0, 0.0, false, false, false, 0),
            ("azalea", "Азалия", 0.0, 0.0, false, false, false, 0),
            ("flowering_azalea", "Цветущая азалия", 0.0, 0.0, false, false, false, 0),
            ("mangrove_propagule", "Мангровый черенок", 0.0, 0.0, false, false, false, 0),
            ("cherry_sapling", "Вишнёвый росток", 0.0, 0.0, false, false, false, 0),
            ("moss_block", "Блок мха", 0.0, 0.0, false, false, false, 0),
            ("moss_carpet", "Ковёр из мха", 0.0, 0.0, false, false, false, 0),
            ("hanging_moss", "Висячий мох", 0.0, 0.0, false, false, false, 0),
            ("cave_vines", "Пещерные лозы", 0.0, 0.0, false, false, false, 0),
            ("cave_vines_berries", "Ягоды пещерных лоз", 0.0, 0.0, false, false, false, 0),
            ("glow_lichen", "Светящийся лишайник", 0.0, 0.0, false, false, true, 1),
            ("spore_blossom", "Споровый цветок", 0.0, 0.0, false, false, false, 0),
        ];

        for (id, name, hardness, blast, opaque, full_cube, transparent, light) in full_cubes {
            registry.register(BlockDefinition {
                id: ResourceId::minecraft(id).unwrap(),
                name: name.to_string(),
                hardness,
                blast_resistance: blast,
                is_opaque: opaque,
                is_full_cube: full_cube,
                is_transparent: transparent,
                emits_light: light,
                block_states: vec![BlockState {
                    properties: HashMap::new(),
                }],
                default_state: BlockState {
                    properties: HashMap::new(),
                },
            });
        }
    }

    fn register_fluids(registry: &mut BlockRegistry) {
        // Жидкости
        let fluids = [
            ("water", "Вода", 0.0, 0.0, false, false, true, 0),
            ("lava", "Лава", 0.0, 0.0, false, false, true, 15),
        ];

        for (id, name, hardness, blast, opaque, full_cube, transparent, light) in fluids {
            registry.register(BlockDefinition {
                id: ResourceId::minecraft(id).unwrap(),
                name: name.to_string(),
                hardness,
                blast_resistance: blast,
                is_opaque: opaque,
                is_full_cube: full_cube,
                is_transparent: transparent,
                emits_light: light,
                block_states: vec![BlockState {
                    properties: HashMap::new(),
                }],
                default_state: BlockState {
                    properties: HashMap::new(),
                },
            });
        }
    }

    fn register_redstone(registry: &mut BlockRegistry) {
        // Редстоун и связанные блоки
        let redstone_blocks = [
            ("redstone_block", "Блок редстоуна", 3.0, 6.0, true, true, false, 0),
            ("redstone_wire", "Редстоун", 0.0, 0.0, false, false, false, 0),
            ("redstone_torch", "Факел редстоуна", 0.0, 0.0, false, false, false, 7),
            ("lever", "Рычаг", 0.5, 0.5, false, false, false, 0),
            ("button", "Кнопка", 0.5, 0.5, false, false, false, 0),
        ];

        for (id, name, hardness, blast, opaque, full_cube, transparent, light) in redstone_blocks {
            registry.register(BlockDefinition {
                id: ResourceId::minecraft(id).unwrap(),
                name: name.to_string(),
                hardness,
                blast_resistance: blast,
                is_opaque: opaque,
                is_full_cube: full_cube,
                is_transparent: transparent,
                emits_light: light,
                block_states: vec![BlockState {
                    properties: HashMap::new(),
                }],
                default_state: BlockState {
                    properties: HashMap::new(),
                },
            });
        }
    }

    fn register_decorations(registry: &mut BlockRegistry) {
        // Декоративные блоки
        let decoration_blocks = [
            ("stone_bricks", "Каменный кирпич", 2.0, 6.0, true, true, false, 0),
            ("cobblestone_stairs", "Каменные ступени", 2.0, 6.0, true, false, false, 0),
            ("oak_fence", "Дубовый забор", 2.0, 3.0, false, false, false, 0),
            ("torch", "Факел", 0.0, 0.0, false, false, false, 7),
            ("glass", "Стекло", 0.3, 0.5, false, false, true, 0),
            ("glass_pane", "Стеклянная панель", 0.3, 0.5, false, false, true, 0),
        ];

        for (id, name, hardness, blast, opaque, full_cube, transparent, light) in decoration_blocks {
            registry.register(BlockDefinition {
                id: ResourceId::minecraft(id).unwrap(),
                name: name.to_string(),
                hardness,
                blast_resistance: blast,
                is_opaque: opaque,
                is_full_cube: full_cube,
                is_transparent: transparent,
                emits_light: light,
                block_states: vec![BlockState {
                    properties: HashMap::new(),
                }],
                default_state: BlockState {
                    properties: HashMap::new(),
                },
            });
        }
    }

    fn register_natural(registry: &mut BlockRegistry) {
        // Естественные блоки
        let natural_blocks = [
            ("grass_block", "Трава", 0.6, 0.6, true, true, false, 0),
            ("dirt", "Глина", 0.5, 0.5, true, true, false, 0),
            ("sand", "Песок", 0.5, 0.5, true, true, false, 0),
            ("gravel", "Гравий", 0.6, 0.6, true, true, false, 0),
            ("stone", "Камень", 1.5, 6.0, true, true, false, 0),
            ("coal_ore", "Угольная руда", 3.0, 3.0, true, true, false, 0),
            ("iron_ore", "Железная руда", 3.0, 3.0, true, true, false, 0),
        ];

        for (id, name, hardness, blast, opaque, full_cube, transparent, light) in natural_blocks {
            registry.register(BlockDefinition {
                id: ResourceId::minecraft(id).unwrap(),
                name: name.to_string(),
                hardness,
                blast_resistance: blast,
                is_opaque: opaque,
                is_full_cube: full_cube,
                is_transparent: transparent,
                emits_light: light,
                block_states: vec![BlockState {
                    properties: HashMap::new(),
                }],
                default_state: BlockState {
                    properties: HashMap::new(),
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_basic_operations() {
        let mut registry = BlockRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        let block = BlockDefinition {
            id: ResourceId::minecraft("test_block").unwrap(),
            name: "Test Block".to_string(),
            hardness: 1.0,
            blast_resistance: 3.0,
            is_opaque: true,
            is_full_cube: true,
            is_transparent: false,
            emits_light: 0,
            block_states: vec![BlockState {
                properties: HashMap::new(),
            }],
            default_state: BlockState {
                properties: HashMap::new(),
            },
        };

        registry.register(block.clone());
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let retrieved = registry.get(&ResourceId::minecraft("test_block").unwrap());
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Block");
    }

    #[test]
    fn standard_blocks_register_all() {
        let mut registry = BlockRegistry::new();
        standard_blocks::register_all(&mut registry);

        assert!(registry.len() > 0);
        assert!(registry.get(&ResourceId::minecraft("stone").unwrap()).is_some());
        assert!(registry.get(&ResourceId::minecraft("dirt").unwrap()).is_some());
        assert!(registry.get(&ResourceId::minecraft("lava").unwrap()).is_some());
        assert!(registry.get(&ResourceId::minecraft("oak_log").unwrap()).is_some());
        assert!(registry.get(&ResourceId::minecraft("poppy").unwrap()).is_some());
    }
}

