use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::types::VoxelId;
use super::Inventory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub inputs: Vec<(VoxelId, u16)>,
    pub output: (VoxelId, u16),
}

#[derive(Resource, Default)]
pub struct RecipeBook(pub Vec<Recipe>);

impl RecipeBook {
    pub fn available(&self, inventory: &Inventory) -> Vec<&Recipe> {
        self.0.iter().filter(|r| can_craft(inventory, r)).collect()
    }
}

pub fn can_craft(inventory: &Inventory, recipe: &Recipe) -> bool {
    recipe.inputs.iter().all(|(voxel_id, needed)| {
        let have: u16 = inventory.slots.iter()
            .filter(|s| s.voxel_id == *voxel_id)
            .map(|s| s.count)
            .sum();
        have >= *needed
    })
}

pub fn apply_craft(inventory: &mut Inventory, recipe: &Recipe) -> bool {
    if !can_craft(inventory, recipe) { return false; }
    for &(voxel_id, count) in &recipe.inputs {
        let mut to_remove = count;
        for slot in &mut inventory.slots {
            if slot.voxel_id == voxel_id && slot.count > 0 && to_remove > 0 {
                let take = to_remove.min(slot.count);
                slot.count -= take;
                to_remove -= take;
                if slot.count == 0 { slot.voxel_id = crate::types::AIR; }
            }
        }
    }
    inventory.add(recipe.output.0, recipe.output.1);
    true
}

pub fn load_recipes(mut recipe_book: ResMut<RecipeBook>) {
    // TODO: load from assets/recipes.ron via AssetServer in a future task
    recipe_book.0 = vec![
        Recipe {
            inputs: vec![(crate::types::STONE, 4)],
            output: (crate::types::STONE, 4),
        },
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::STONE;
    use crate::inventory::Inventory;

    fn pickaxe_recipe() -> Recipe {
        Recipe {
            inputs: vec![(STONE, 3)],
            output: (STONE, 1),
        }
    }

    #[test]
    fn can_craft_when_enough_ingredients() {
        let mut inv = Inventory::default();
        inv.add(STONE, 5);
        assert!(can_craft(&inv, &pickaxe_recipe()));
    }

    #[test]
    fn cannot_craft_when_insufficient() {
        let mut inv = Inventory::default();
        inv.add(STONE, 2);
        assert!(!can_craft(&inv, &pickaxe_recipe()));
    }

    #[test]
    fn apply_craft_consumes_inputs() {
        let mut inv = Inventory::default();
        inv.add(STONE, 5);
        let ok = apply_craft(&mut inv, &pickaxe_recipe());
        assert!(ok);
        let stone_count: u16 = inv.slots.iter().filter(|s| s.voxel_id == STONE).map(|s| s.count).sum();
        assert_eq!(stone_count, 3);
    }

    #[test]
    fn apply_craft_fails_when_insufficient() {
        let mut inv = Inventory::default();
        inv.add(STONE, 1);
        let ok = apply_craft(&mut inv, &pickaxe_recipe());
        assert!(!ok);
        let stone_count: u16 = inv.slots.iter().filter(|s| s.voxel_id == STONE).map(|s| s.count).sum();
        assert_eq!(stone_count, 1, "inventory unchanged on failure");
    }
}
