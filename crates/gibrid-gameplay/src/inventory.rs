//! Инвентарь — хранение предметов в слотах.
//! Поддерживает игроков, сундуки, шкафчики и другие контейнеры.

use gibrid_core::ResourceId;
use std::collections::HashMap;

/// Слот инвентаря — предмет + количество.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub item_id: ResourceId,
    pub count: u32,
    pub components: ItemComponents,
}

/// Компоненты предмета в инвентаре (NBT-подобные данные).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemComponents {
    pub custom_name: Option<String>,
    pub enchantments: Vec<(String, i32)>,
    pub attributes: Vec<(String, i64, i32)>, // Используем i64 вместо f64 для Eq
}

/// Тип контейнера инвентаря.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerType {
    Player,     // Инвентарь игрока (36 слотов + экипировка)
    Chest,      // Сундук (27 или 54 слота)
    Barrel,     // Бочка (27 слотов)
    ShulkerBox, // Шалкер-бокс (27 слотов)
    Hopper,     // Воронка (5 слотов)
    Furnace,    // Печь (3 слота: вход, топливо, выход)
    CraftingTable, // Стол крафта (9 слотов + результат)
}

/// Описание контейнера: сколько слотов и какой тип.
#[derive(Debug, Clone)]
pub struct ContainerDefinition {
    pub container_type: ContainerType,
    pub slot_count: usize,
    pub name: String,
}

impl ContainerDefinition {
    pub fn player_inventory() -> Self {
        Self {
            container_type: ContainerType::Player,
            slot_count: 36,
            name: "Инвентарь".to_string(),
        }
    }

    pub fn chest() -> Self {
        Self {
            container_type: ContainerType::Chest,
            slot_count: 27,
            name: "Сундук".to_string(),
        }
    }

    pub fn barrel() -> Self {
        Self {
            container_type: ContainerType::Barrel,
            slot_count: 27,
            name: "Бочка".to_string(),
        }
    }

    pub fn hopper() -> Self {
        Self {
            container_type: ContainerType::Hopper,
            slot_count: 5,
            name: "Воронка".to_string(),
        }
    }

    pub fn furnace() -> Self {
        Self {
            container_type: ContainerType::Furnace,
            slot_count: 3,
            name: "Печь".to_string(),
        }
    }

    pub fn crafting_table() -> Self {
        Self {
            container_type: ContainerType::CraftingTable,
            slot_count: 10, // 9 для крафта + 1 результат
            name: "Стол крафта".to_string(),
        }
    }
}

/// Инвентарь — набор слотов с предметами.
#[derive(Debug, Clone)]
pub struct Inventory {
    pub definition: ContainerDefinition,
    pub slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn new(definition: ContainerDefinition) -> Self {
        let slot_count = definition.slot_count;
        Self {
            definition,
            slots: vec![None; slot_count],
        }
    }

    /// Добавить предмет в инвентарь. Возвращает количество неразмещённых предметов.
    pub fn add_item(&mut self, item_id: ResourceId, count: u32) -> u32 {
        let mut remaining = count;

        // Сначала попробуем сложить с существующими стэками того же предмета
        for slot in self.slots.iter_mut() {
            if remaining == 0 {
                break;
            }
            if let Some(stack) = slot {
                if stack.item_id == item_id && stack.count < 64 {
                    let space = 64 - stack.count;
                    let to_add = remaining.min(space);
                    stack.count += to_add;
                    remaining -= to_add;
                }
            }
        }

        // Затем в пустые слоты
        for slot in self.slots.iter_mut() {
            if remaining == 0 {
                break;
            }
            if slot.is_none() {
                let to_add = remaining.min(64);
                *slot = Some(ItemStack {
                    item_id: item_id.clone(),
                    count: to_add,
                    components: Default::default(),
                });
                remaining -= to_add;
            }
        }

        remaining
    }

    /// Удалить предмет из инвентаря. Возвращает количество удалённых.
    pub fn remove_item(&mut self, item_id: &ResourceId, count: u32) -> u32 {
        let mut remaining = count;

        for slot in self.slots.iter_mut() {
            if remaining == 0 {
                break;
            }
            if let Some(stack) = slot {
                if stack.item_id == *item_id {
                    let to_remove = remaining.min(stack.count);
                    stack.count -= to_remove;
                    remaining -= to_remove;
                    if stack.count == 0 {
                        *slot = None;
                    }
                }
            }
        }

        count - remaining
    }

    /// Проверить, есть ли предмет в инвентаре.
    pub fn has_item(&self, item_id: &ResourceId) -> bool {
        self.slots.iter().any(|s| matches!(s, Some(stack) if stack.item_id == *item_id))
    }

    /// Получить количество предмета в инвентаре.
    pub fn item_count(&self, item_id: &ResourceId) -> u32 {
        self.slots.iter()
            .filter_map(|s| s.as_ref())
            .filter(|stack| stack.item_id == *item_id)
            .map(|stack| stack.count)
            .sum()
    }

    /// Перенести предмет из одного слота в другой.
    pub fn transfer(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from >= self.slots.len() || to >= self.slots.len() {
            return Err("Invalid slot index".to_string());
        }

        let item = self.slots[from].take();
        if let Some(mut stack) = item {
            if let Some(existing) = &mut self.slots[to] {
                if existing.item_id == stack.item_id && existing.count < 64 {
                    let space = 64 - existing.count;
                    let to_move = stack.count.min(space);
                    existing.count += to_move;
                    stack.count -= to_move;
                    if stack.count > 0 {
                        self.slots[from] = Some(stack);
                    }
                } else {
                    // Нельзя сложить, вернуть обратно
                    self.slots[from] = Some(stack);
                    return Err("Cannot stack items".to_string());
                }
            } else {
                self.slots[to] = Some(stack);
            }
        }

        Ok(())
    }

    /// Получить ссылку на слот.
    pub fn get_slot(&self, index: usize) -> Option<&Option<ItemStack>> {
        self.slots.get(index)
    }

    /// Установить слот.
    pub fn set_slot(&mut self, index: usize, item: Option<ItemStack>) -> Result<(), String> {
        if index >= self.slots.len() {
            return Err("Invalid slot index".to_string());
        }
        self.slots[index] = item;
        Ok(())
    }

    /// Очистить слот.
    pub fn clear_slot(&mut self, index: usize) -> Result<Option<ItemStack>, String> {
        if index >= self.slots.len() {
            return Err("Invalid slot index".to_string());
        }
        Ok(self.slots[index].take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_add_and_remove() {
        let mut inv = Inventory::new(ContainerDefinition::player_inventory());
        let stone = ResourceId::minecraft("stone").unwrap();

        // Добавить предмет
        let remaining = inv.add_item(stone.clone(), 10);
        assert_eq!(remaining, 0);
        assert!(inv.has_item(&stone));
        assert_eq!(inv.item_count(&stone), 10);

        // Удалить часть
        let removed = inv.remove_item(&stone, 3);
        assert_eq!(removed, 3);
        assert_eq!(inv.item_count(&stone), 7);
    }

    #[test]
    fn inventory_stack_limit() {
        let mut inv = Inventory::new(ContainerDefinition::player_inventory());
        let stone = ResourceId::minecraft("stone").unwrap();

        // Добавить больше 64 - должно заполнить первый слот на 64, остаток во второй
        let remaining = inv.add_item(stone.clone(), 100);
        assert_eq!(remaining, 0); // Все 100 поместились: 64 + 36
        assert_eq!(inv.item_count(&stone), 100);
    }

    #[test]
    fn inventory_transfer() {
        let mut inv = Inventory::new(ContainerDefinition::player_inventory());
        let stone = ResourceId::minecraft("stone").unwrap();

        inv.add_item(stone.clone(), 10);
        inv.transfer(0, 5).unwrap();

        assert!(inv.get_slot(5).unwrap().is_some());
        assert!(inv.get_slot(0).unwrap().is_none());
        assert_eq!(inv.item_count(&stone), 10);
    }

    #[test]
    fn container_definitions() {
        let player_inv = ContainerDefinition::player_inventory();
        assert_eq!(player_inv.slot_count, 36);

        let chest = ContainerDefinition::chest();
        assert_eq!(chest.slot_count, 27);

        let furnace = ContainerDefinition::furnace();
        assert_eq!(furnace.slot_count, 3);
    }
}
