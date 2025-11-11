// src/world/inventory.rs
//! Inventory and item management system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemType {
    // Raw resources
    Wood,
    Stone,
    Iron,
    Food,

    // Tools
    WoodenAxe,
    StoneAxe,
    IronAxe,
    WoodenPickaxe,
    StonePickaxe,
    IronPickaxe,

    // Weapons/Armor
    WoodenSpear,
    StoneSpear,
    IronSword,
    LeatherArmor,
    IronArmor,
    SteelArmor,
}

impl ItemType {
    /// Check if item is a tool
    pub fn is_tool(&self) -> bool {
        matches!(
            self,
            ItemType::WoodenAxe | ItemType::StoneAxe | ItemType::IronAxe |
            ItemType::WoodenPickaxe | ItemType::StonePickaxe | ItemType::IronPickaxe
        )
    }

    /// Check if item is a weapon
    pub fn is_weapon(&self) -> bool {
        matches!(
            self,
            ItemType::WoodenSpear | ItemType::StoneSpear | ItemType::IronSword
        )
    }

    /// Check if item is armor
    pub fn is_armor(&self) -> bool {
        matches!(
            self,
            ItemType::LeatherArmor | ItemType::IronArmor | ItemType::SteelArmor
        )
    }

    /// Get tool efficiency multiplier (1.0 = base)
    pub fn efficiency(&self) -> f32 {
        match self {
            ItemType::IronAxe | ItemType::IronPickaxe => 2.0,   // 2x faster
            ItemType::StoneAxe | ItemType::StonePickaxe => 1.5, // 1.5x faster
            ItemType::WoodenAxe | ItemType::WoodenPickaxe => 1.2, // 1.2x faster
            _ => 1.0,
        }
    }

    /// Get base durability (uses before breaking, 0 = infinite)
    pub fn durability(&self) -> u32 {
        match self {
            // Tools wear out
            ItemType::WoodenAxe | ItemType::WoodenPickaxe => 50,
            ItemType::StoneAxe | ItemType::StonePickaxe => 100,
            ItemType::IronAxe | ItemType::IronPickaxe => 200,

            // Weapons/armor wear out
            ItemType::WoodenSpear => 30,
            ItemType::StoneSpear => 60,
            ItemType::IronSword => 150,
            ItemType::LeatherArmor => 80,
            ItemType::IronArmor => 200,
            ItemType::SteelArmor => 300,

            // Resources don't wear out
            _ => 0,
        }
    }
}

/// An item instance with durability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub item_type: ItemType,
    pub quantity: u32, // Stack size (for resources)
    pub durability: u32, // Remaining uses (for tools/equipment)
    pub max_durability: u32,
}

impl Item {
    pub fn new(item_type: ItemType, quantity: u32) -> Self {
        let max_durability = item_type.durability();
        Self {
            item_type,
            quantity,
            durability: max_durability,
            max_durability,
        }
    }

    /// Use the item (reduce durability)
    pub fn use_item(&mut self) -> bool {
        if self.max_durability > 0 {
            if self.durability > 0 {
                self.durability -= 1;
                return self.durability > 0; // True if still usable
            }
            return false; // Broken
        }
        true // Items with 0 max durability never break
    }

    pub fn is_broken(&self) -> bool {
        self.max_durability > 0 && self.durability == 0
    }

    pub fn durability_percentage(&self) -> f32 {
        if self.max_durability == 0 {
            return 100.0;
        }
        (self.durability as f32 / self.max_durability as f32) * 100.0
    }
}

/// Inventory for storing items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub items: HashMap<ItemType, Item>,
    pub capacity: u32, // Maximum total quantity
}

impl Inventory {
    pub fn new(capacity: u32) -> Self {
        Self {
            items: HashMap::new(),
            capacity,
        }
    }

    /// Get total item count
    pub fn total_count(&self) -> u32 {
        self.items.values().map(|item| item.quantity).sum()
    }

    /// Check if inventory has space
    pub fn has_space(&self, quantity: u32) -> bool {
        self.total_count() + quantity <= self.capacity
    }

    /// Add items to inventory
    pub fn add_item(&mut self, item_type: ItemType, quantity: u32) -> bool {
        if !self.has_space(quantity) {
            return false;
        }

        self.items
            .entry(item_type)
            .and_modify(|item| item.quantity += quantity)
            .or_insert_with(|| Item::new(item_type, quantity));

        true
    }

    /// Remove items from inventory
    pub fn remove_item(&mut self, item_type: &ItemType, quantity: u32) -> bool {
        if let Some(item) = self.items.get_mut(item_type) {
            if item.quantity >= quantity {
                item.quantity -= quantity;

                if item.quantity == 0 {
                    self.items.remove(item_type);
                }

                return true;
            }
        }
        false
    }

    /// Check if inventory has items
    pub fn has_item(&self, item_type: &ItemType, quantity: u32) -> bool {
        self.items
            .get(item_type)
            .map(|item| item.quantity >= quantity)
            .unwrap_or(false)
    }

    /// Get item count
    pub fn count_item(&self, item_type: &ItemType) -> u32 {
        self.items
            .get(item_type)
            .map(|item| item.quantity)
            .unwrap_or(0)
    }

    /// Get all items
    pub fn list_items(&self) -> Vec<(&ItemType, &Item)> {
        self.items.iter().collect()
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new(100) // Default capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_creation() {
        let item = Item::new(ItemType::IronAxe, 1);
        assert_eq!(item.item_type, ItemType::IronAxe);
        assert_eq!(item.quantity, 1);
        assert_eq!(item.durability, 200);
        assert_eq!(item.max_durability, 200);
    }

    #[test]
    fn test_item_durability() {
        let mut item = Item::new(ItemType::WoodenAxe, 1);
        assert!(!item.is_broken());

        // Use until broken
        for _ in 0..50 {
            item.use_item();
        }

        assert!(item.is_broken());
    }

    #[test]
    fn test_inventory_add_remove() {
        let mut inv = Inventory::new(100);

        assert!(inv.add_item(ItemType::Wood, 10));
        assert_eq!(inv.count_item(&ItemType::Wood), 10);

        assert!(inv.add_item(ItemType::Wood, 5));
        assert_eq!(inv.count_item(&ItemType::Wood), 15);

        assert!(inv.remove_item(&ItemType::Wood, 10));
        assert_eq!(inv.count_item(&ItemType::Wood), 5);

        assert!(inv.remove_item(&ItemType::Wood, 5));
        assert_eq!(inv.count_item(&ItemType::Wood), 0);
    }

    #[test]
    fn test_inventory_capacity() {
        let mut inv = Inventory::new(10);

        assert!(inv.add_item(ItemType::Stone, 5));
        assert!(inv.add_item(ItemType::Wood, 5));
        assert!(!inv.add_item(ItemType::Iron, 1)); // Over capacity

        assert_eq!(inv.total_count(), 10);
    }

    #[test]
    fn test_inventory_has_item() {
        let mut inv = Inventory::new(100);

        inv.add_item(ItemType::Food, 20);

        assert!(inv.has_item(&ItemType::Food, 10));
        assert!(inv.has_item(&ItemType::Food, 20));
        assert!(!inv.has_item(&ItemType::Food, 21));
        assert!(!inv.has_item(&ItemType::Wood, 1));
    }
}
