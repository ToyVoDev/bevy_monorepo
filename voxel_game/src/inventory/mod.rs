pub mod crafting;
pub mod ui;

use bevy::prelude::*;
use crate::types::{VoxelId, AIR};

pub const INVENTORY_SIZE: usize = 36;
pub const HOTBAR_SIZE: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    Hand,
    Pickaxe,
    Shovel,
}

impl ToolType {
    pub fn can_break(self, voxel_id: VoxelId) -> bool {
        match self {
            ToolType::Hand => voxel_id == crate::types::DIRT || voxel_id == crate::types::TOPSOIL,
            ToolType::Pickaxe => true,
            ToolType::Shovel => voxel_id == crate::types::DIRT || voxel_id == crate::types::TOPSOIL,
        }
    }

    pub fn debris_count(self) -> u8 {
        match self {
            ToolType::Hand => 1,
            ToolType::Pickaxe => 3,
            ToolType::Shovel => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InventorySlot {
    pub voxel_id: VoxelId,
    pub count: u16,
}

#[derive(Resource)]
pub struct Inventory {
    pub slots: [InventorySlot; INVENTORY_SIZE],
    pub active_slot: usize,
    pub active_tool: ToolType,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: [InventorySlot::default(); INVENTORY_SIZE],
            active_slot: 0,
            active_tool: ToolType::Hand,
        }
    }
}

impl Inventory {
    pub fn add(&mut self, voxel_id: VoxelId, count: u16) -> u16 {
        for slot in &mut self.slots {
            if slot.voxel_id == voxel_id {
                // saturating_add: stack overflow silently caps at u16::MAX; a future task
                // should enforce a per-slot stack cap and return true leftover.
                slot.count = slot.count.saturating_add(count);
                return 0;
            }
        }
        for slot in &mut self.slots {
            if slot.voxel_id == AIR {
                slot.voxel_id = voxel_id;
                slot.count = count;
                return 0;
            }
        }
        count
    }

    pub fn remove(&mut self, slot: usize, count: u16) -> bool {
        if slot >= INVENTORY_SIZE {
            return false;
        }
        if self.slots[slot].count < count {
            return false;
        }
        self.slots[slot].count -= count;
        if self.slots[slot].count == 0 {
            self.slots[slot].voxel_id = AIR;
        }
        true
    }

    pub fn active_voxel_id(&self) -> VoxelId {
        self.slots[self.active_slot].voxel_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::STONE;

    #[test]
    fn add_to_empty_inventory() {
        let mut inv = Inventory::default();
        let leftover = inv.add(STONE, 5);
        assert_eq!(leftover, 0);
        assert_eq!(inv.slots[0].voxel_id, STONE);
        assert_eq!(inv.slots[0].count, 5);
    }

    #[test]
    fn add_stacks_with_existing() {
        let mut inv = Inventory::default();
        inv.add(STONE, 5);
        inv.add(STONE, 3);
        assert_eq!(inv.slots[0].count, 8, "should stack into same slot");
    }

    #[test]
    fn remove_decrements_count() {
        let mut inv = Inventory::default();
        inv.add(STONE, 5);
        let ok = inv.remove(0, 3);
        assert!(ok);
        assert_eq!(inv.slots[0].count, 2);
    }

    #[test]
    fn remove_clears_empty_slot() {
        let mut inv = Inventory::default();
        inv.add(STONE, 3);
        inv.remove(0, 3);
        assert_eq!(inv.slots[0].voxel_id, AIR);
        assert_eq!(inv.slots[0].count, 0);
    }

    #[test]
    fn remove_fails_when_insufficient() {
        let mut inv = Inventory::default();
        inv.add(STONE, 2);
        let ok = inv.remove(0, 5);
        assert!(!ok);
        assert_eq!(inv.slots[0].count, 2, "count unchanged on failure");
    }
}

#[cfg(test)]
mod tool_tests {
    use super::*;
    use crate::types::{STONE, DIRT, TOPSOIL};

    #[test]
    fn pickaxe_can_break_stone() {
        assert!(ToolType::Pickaxe.can_break(STONE));
    }

    #[test]
    fn hand_cannot_break_stone() {
        assert!(!ToolType::Hand.can_break(STONE));
    }

    #[test]
    fn hand_can_break_dirt() {
        assert!(ToolType::Hand.can_break(DIRT));
        assert!(ToolType::Hand.can_break(TOPSOIL));
    }

    #[test]
    fn pickaxe_ejects_three_debris() {
        assert_eq!(ToolType::Pickaxe.debris_count(), 3);
    }

    #[test]
    fn hand_ejects_one_debris() {
        assert_eq!(ToolType::Hand.debris_count(), 1);
    }
}
