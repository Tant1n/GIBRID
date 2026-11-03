// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Рецепты крафта и менеджер крафта.
//! Статус: скелет полноценного ядра. Базовые рецепты: камень → щебень, древесина → доски и т.д.

use gibrid_core::ResourceId;
use std::collections::HashMap;

/// Один ингредиент рецепта: слот в сетке крафта (3x3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeIngredient {
    pub slot: usize,   // 0..8 в сетке 3x3
    pub item_id: ResourceId,
}

/// Один рецепт крафта.
#[derive(Debug, Clone)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub ingredients: Vec<RecipeIngredient>,
    pub result_item: ResourceId,
    pub result_count: u32,
}

/// Менеджер рецептов крафта.
pub struct CraftingManager {
    recipes: Vec<Recipe>,
    by_result: HashMap<ResourceId, Vec<usize>>, // индексы рецептов по результату
}

impl Default for CraftingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CraftingManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            recipes: Vec::new(),
            by_result: HashMap::new(),
        };
        mgr.register_basic_recipes();
        mgr
    }

    fn register_basic_recipes(&mut self) {
        let stone = ResourceId::minecraft("stone").unwrap();
        let cobblestone = ResourceId::minecraft("cobblestone").unwrap();

        // Камень → щебень (1 камень в любом слоте сетки)
        self.add_recipe(Recipe {
            id: "stone_to_cobblestone".to_string(),
            name: "Камень → Щебень".to_string(),
            ingredients: vec![RecipeIngredient { slot: 0, item_id: stone.clone() }],
            result_item: cobblestone.clone(),
            result_count: 1,
        });

        // 4 щебня → 1 камень (сетка 2x2)
        self.add_recipe(Recipe {
            id: "cobblestone_to_stone".to_string(),
            name: "Щебень → Камень".to_string(),
            ingredients: vec![
                RecipeIngredient { slot: 0, item_id: cobblestone.clone() },
                RecipeIngredient { slot: 1, item_id: cobblestone.clone() },
                RecipeIngredient { slot: 3, item_id: cobblestone.clone() },
                RecipeIngredient { slot: 4, item_id: cobblestone.clone() },
            ],
            result_item: stone,
            result_count: 1,
        });
    }

    fn add_recipe(&mut self, recipe: Recipe) {
        let idx = self.recipes.len();
        self.by_result.entry(recipe.result_item.clone()).or_default().push(idx);
        self.recipes.push(recipe);
    }

    /// Попробовать собрать рецепт из сетки крафта (9 слотов).
    /// grid[i] — предмет в слоте i (0..8) или None.
    /// Возвращает (рецепт, использованные слоты) или None.
    pub fn craft(&self, grid: &[Option<ResourceId>; 9]) -> Option<(usize, Vec<usize>)> {
        for (idx, recipe) in self.recipes.iter().enumerate() {
            let mut used_slots = Vec::new();
            let mut ok = true;

            for ing in &recipe.ingredients {
                if let Some(item) = &grid[ing.slot] {
                    if *item == ing.item_id {
                        used_slots.push(ing.slot);
                    } else {
                        ok = false;
                        break;
                    }
                } else {
                    ok = false;
                    break;
                }
            }

            if ok && used_slots.len() == recipe.ingredients.len() {
                return Some((idx, used_slots));
            }
        }
        None
    }

    /// Получить рецепт по индексу.
    pub fn get_recipe(&self, idx: usize) -> Option<&Recipe> {
        self.recipes.get(idx)
    }

    /// Все рецепты, дающие данный предмет.
    pub fn recipes_for(&self, item_id: &ResourceId) -> Vec<&Recipe> {
        self.by_result
            .get(item_id)
            .map(|idxs| idxs.iter().filter_map(|i| self.recipes.get(*i)).collect())
            .unwrap_or_default()
    }

    /// Количество зарегистрированных рецептов.
    pub fn len(&self) -> usize {
        self.recipes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn craft_stone_to_cobblestone() {
        let mgr = CraftingManager::default();
        let mut grid = [None; 9];
        let stone = ResourceId::minecraft("stone").unwrap();
        grid[0] = Some(stone.clone());

        let result = mgr.craft(&grid);
        assert!(result.is_some());
        let (idx, used) = result.unwrap();
        let recipe = mgr.get_recipe(idx).unwrap();
        assert_eq!(recipe.result_item, ResourceId::minecraft("cobblestone").unwrap());
        assert_eq!(used, vec![0]);
    }

    #[test]
    fn craft_cobblestone_to_stone() {
        let mgr = CraftingManager::default();
        let cobblestone = ResourceId::minecraft("cobblestone").unwrap();
        let mut grid = [None; 9];
        for s in [0, 1, 3, 4] {
            grid[s] = Some(cobblestone.clone());
        }

        let result = mgr.craft(&grid);
        assert!(result.is_some());
        let (idx, _) = result.unwrap();
        let recipe = mgr.get_recipe(idx).unwrap();
        assert_eq!(recipe.result_item, stone());
    }

    fn stone() -> ResourceId {
        ResourceId::minecraft("stone").unwrap()
    }

    #[test]
    fn craft_no_match() {
        let mgr = CraftingManager::default();
        let mut grid = [None; 9];
        grid[0] = Some(ResourceId::minecraft("dirt").unwrap());
        assert!(mgr.craft(&grid).is_none());
    }

    #[test]
    fn recipes_for_item() {
        let mgr = CraftingManager::default();
        let recipes = mgr.recipes_for(&ResourceId::minecraft("cobblestone").unwrap());
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].id, "stone_to_cobblestone");
    }
}
