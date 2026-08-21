// src/agents/storage_integration.rs
//! Integration layer between agent inventory and world storehouse.
//!
//! Bridges the gap between the two inventory systems:
//! - Agent: HashMap<String, InventoryItem> (string-based IDs, weight tracking)
//! - World: HashMap<ItemType, Item> (enum-based, simple quantity)

use crate::world::ItemType;
use super::agent::{Inventory, InventoryItem};

/// Convert ItemType to string ID for agent inventory
pub fn item_type_to_id(item_type: ItemType) -> String {
    format!("{:?}", item_type).to_lowercase()
}

/// Strip the preparation prefix from an item id.
///
/// Food that has been over a fire is carried under its own id - `cooked_fish`,
/// `burnt_meat` - because one inventory stack can hold only one preparation
/// state. Underneath it is still fish and still meat.
pub fn base_item_id(id: &str) -> &str {
    id.strip_prefix("cooked_")
        .or_else(|| id.strip_prefix("burnt_"))
        .unwrap_or(id)
}

/// Convert string ID to ItemType (best effort)
pub fn id_to_item_type(id: &str) -> Option<ItemType> {
    match base_item_id(&id.to_lowercase()) {
        // Basic Resources
        "wood" => Some(ItemType::Wood),
        "stone" => Some(ItemType::Stone),
        "iron" => Some(ItemType::Iron),
        "food" => Some(ItemType::Food),

        // Agricultural
        "grain" => Some(ItemType::Grain),
        "flax" => Some(ItemType::Flax),
        "herbs" => Some(ItemType::Herbs),
        "cotton" => Some(ItemType::Cotton),

        // Animal
        "hides" => Some(ItemType::Hides),
        "wool" => Some(ItemType::Wool),
        "meat" => Some(ItemType::Meat),
        "milk" => Some(ItemType::Milk),
        "fish" => Some(ItemType::Fish),
        "honey" => Some(ItemType::Honey),

        // Mineral
        "clay" => Some(ItemType::Clay),
        "sand" => Some(ItemType::Sand),
        "coal" => Some(ItemType::Coal),

        // Processed
        "flour" => Some(ItemType::Flour),
        "leather" => Some(ItemType::Leather),
        "cloth" => Some(ItemType::Cloth),
        "linen" => Some(ItemType::Linen),
        "glass" => Some(ItemType::Glass),
        "bricks" => Some(ItemType::Bricks),
        "charcoal" => Some(ItemType::Charcoal),
        "rope" => Some(ItemType::Rope),
        "paper" => Some(ItemType::Paper),
        "dye" => Some(ItemType::Dye),

        // Finished Food
        "bread" => Some(ItemType::Bread),
        "ale" => Some(ItemType::Ale),
        "cheese" => Some(ItemType::Cheese),

        // Finished Goods
        "clothing" => Some(ItemType::Clothing),
        "shoes" => Some(ItemType::Shoes),
        "pottery" => Some(ItemType::Pottery),
        "furniture" => Some(ItemType::Furniture),
        "jewelry" => Some(ItemType::Jewelry),

        // Tools
        "woodenaxe" | "wooden_axe" => Some(ItemType::WoodenAxe),
        "stoneaxe" | "stone_axe" => Some(ItemType::StoneAxe),
        "ironaxe" | "iron_axe" => Some(ItemType::IronAxe),
        "woodenpickaxe" | "wooden_pickaxe" => Some(ItemType::WoodenPickaxe),
        "stonepickaxe" | "stone_pickaxe" => Some(ItemType::StonePickaxe),
        "ironpickaxe" | "iron_pickaxe" => Some(ItemType::IronPickaxe),
        "woodenhammer" | "wooden_hammer" => Some(ItemType::WoodenHammer),
        "stonehammer" | "stone_hammer" => Some(ItemType::StoneHammer),
        "ironhammer" | "iron_hammer" => Some(ItemType::IronHammer),

        // Weapons
        "woodenspear" | "wooden_spear" => Some(ItemType::WoodenSpear),
        "woodenbow" | "wooden_bow" => Some(ItemType::WoodenBow),
        "stonespear" | "stone_spear" => Some(ItemType::StoneSpear),
        "ironsword" | "iron_sword" => Some(ItemType::IronSword),
        "ironbow" | "iron_bow" => Some(ItemType::IronBow),
        "steelsword" | "steel_sword" => Some(ItemType::SteelSword),

        // Armor
        "leatherarmor" | "leather_armor" => Some(ItemType::LeatherArmor),
        "ironarmor" | "iron_armor" => Some(ItemType::IronArmor),
        "steelarmor" | "steel_armor" => Some(ItemType::SteelArmor),

        _ => None,
    }
}

/// Get weight for an item type
pub fn item_weight(item_type: ItemType) -> f32 {
    match item_type {
        // Light items
        ItemType::Food | ItemType::Bread | ItemType::Cheese => 0.5,
        ItemType::Herbs | ItemType::Paper | ItemType::Dye => 0.2,
        ItemType::Cloth | ItemType::Linen | ItemType::Clothing => 0.3,

        // Medium items
        ItemType::Wood | ItemType::Grain | ItemType::Flax | ItemType::Cotton => 1.0,
        ItemType::Hides | ItemType::Wool | ItemType::Leather => 1.5,
        ItemType::Meat | ItemType::Fish => 0.8,

        // Heavy items
        ItemType::Stone | ItemType::Clay | ItemType::Bricks => 2.0,
        ItemType::Iron | ItemType::Coal | ItemType::Charcoal => 2.5,

        // Tools - medium-heavy
        ItemType::WoodenAxe | ItemType::WoodenPickaxe | ItemType::WoodenHammer => 1.5,
        ItemType::StoneAxe | ItemType::StonePickaxe | ItemType::StoneHammer => 2.0,
        ItemType::IronAxe | ItemType::IronPickaxe | ItemType::IronHammer => 2.5,

        // Weapons
        ItemType::WoodenSpear | ItemType::WoodenBow => 1.0,
        ItemType::StoneSpear => 1.5,
        ItemType::IronSword | ItemType::IronBow | ItemType::SteelSword => 2.0,

        // Armor - heavy
        ItemType::LeatherArmor => 3.0,
        ItemType::IronArmor => 5.0,
        ItemType::SteelArmor => 6.0,

        // Other
        _ => 1.0,
    }
}

/// Try to remove items from agent inventory
/// Returns (success, actual_amount_removed)
pub fn take_from_agent_inventory(
    inventory: &mut Inventory,
    item_type: ItemType,
    amount: u32,
) -> (bool, u32) {
    let item_id = item_type_to_id(item_type);

    if let Some(removed_item) = inventory.remove_item(&item_id, amount) {
        (true, removed_item.quantity)
    } else {
        (false, 0)
    }
}

/// Try to add items to agent inventory
/// Returns (success, actual_amount_added)
pub fn add_to_agent_inventory(
    inventory: &mut Inventory,
    item_type: ItemType,
    amount: u32,
) -> (bool, u32) {
    let item_id = item_type_to_id(item_type);
    let weight = item_weight(item_type);

    let item = InventoryItem::new_with_weight(item_id, amount, weight);

    if inventory.add_item(item) {
        (true, amount)
    } else {
        // Try to add as much as possible
        let mut added = 0;
        for _i in 1..=amount {
            let partial_item = InventoryItem::new_with_weight(
                item_type_to_id(item_type),
                1,
                weight,
            );

            if inventory.add_item(partial_item) {
                added += 1;
            } else {
                break;
            }
        }

        (added == amount, added)
    }
}

/// Count specific item type in agent inventory
pub fn count_in_agent_inventory(inventory: &Inventory, item_type: ItemType) -> u32 {
    let item_id = item_type_to_id(item_type);
    inventory.get_item(&item_id)
        .map(|item| item.quantity)
        .unwrap_or(0)
}

/// Count all food items in agent inventory
pub fn count_food_in_inventory(inventory: &Inventory) -> u32 {
    let food_types = vec![
        ItemType::Food,
        ItemType::Bread,
        ItemType::Cheese,
        ItemType::Meat,
        ItemType::Fish,
        ItemType::Honey,
        ItemType::Ale,
    ];

    food_types.iter()
        .map(|&item_type| count_in_agent_inventory(inventory, item_type))
        .sum()
}

/// Count all resource items (wood, stone, iron) in agent inventory
pub fn count_resources_in_inventory(inventory: &Inventory) -> u32 {
    let resource_types = vec![
        ItemType::Wood,
        ItemType::Stone,
        ItemType::Iron,
        ItemType::Clay,
        ItemType::Sand,
        ItemType::Coal,
    ];

    resource_types.iter()
        .map(|&item_type| count_in_agent_inventory(inventory, item_type))
        .sum()
}

/// Count all tools in agent inventory
pub fn count_tools_in_inventory(inventory: &Inventory) -> u32 {
    let tool_types = vec![
        ItemType::WoodenAxe,
        ItemType::StoneAxe,
        ItemType::IronAxe,
        ItemType::WoodenPickaxe,
        ItemType::StonePickaxe,
        ItemType::IronPickaxe,
        ItemType::WoodenHammer,
        ItemType::StoneHammer,
        ItemType::IronHammer,
    ];

    tool_types.iter()
        .map(|&item_type| count_in_agent_inventory(inventory, item_type))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_type_conversion() {
        let id = item_type_to_id(ItemType::Wood);
        assert_eq!(id, "wood");

        let item_type = id_to_item_type("wood");
        assert_eq!(item_type, Some(ItemType::Wood));
    }

    #[test]
    fn test_item_type_conversion_tools() {
        assert_eq!(id_to_item_type("ironaxe"), Some(ItemType::IronAxe));
        assert_eq!(id_to_item_type("iron_axe"), Some(ItemType::IronAxe));
        assert_eq!(id_to_item_type("woodenspear"), Some(ItemType::WoodenSpear));
    }

    #[test]
    fn test_item_weight() {
        assert_eq!(item_weight(ItemType::Food), 0.5);
        assert_eq!(item_weight(ItemType::Stone), 2.0);
        assert!(item_weight(ItemType::IronArmor) > 4.0);
    }

    #[test]
    fn test_add_to_agent_inventory() {
        let mut inventory = Inventory::new(20, 100.0);

        let (success, added) = add_to_agent_inventory(&mut inventory, ItemType::Food, 10);
        assert!(success);
        assert_eq!(added, 10);

        assert_eq!(count_in_agent_inventory(&inventory, ItemType::Food), 10);
    }

    #[test]
    fn test_take_from_agent_inventory() {
        let mut inventory = Inventory::new(20, 100.0);

        // Add items first
        add_to_agent_inventory(&mut inventory, ItemType::Wood, 20);

        // Remove some
        let (success, removed) = take_from_agent_inventory(&mut inventory, ItemType::Wood, 15);
        assert!(success);
        assert_eq!(removed, 15);

        assert_eq!(count_in_agent_inventory(&inventory, ItemType::Wood), 5);
    }

    #[test]
    fn test_count_food_in_inventory() {
        let mut inventory = Inventory::new(20, 100.0);

        add_to_agent_inventory(&mut inventory, ItemType::Food, 5);
        add_to_agent_inventory(&mut inventory, ItemType::Bread, 3);
        add_to_agent_inventory(&mut inventory, ItemType::Meat, 2);

        assert_eq!(count_food_in_inventory(&inventory), 10);
    }

    #[test]
    fn test_count_resources_in_inventory() {
        let mut inventory = Inventory::new(20, 100.0);

        add_to_agent_inventory(&mut inventory, ItemType::Wood, 10);
        add_to_agent_inventory(&mut inventory, ItemType::Stone, 5);
        add_to_agent_inventory(&mut inventory, ItemType::Iron, 3);

        assert_eq!(count_resources_in_inventory(&inventory), 18);
    }

    #[test]
    fn test_count_tools_in_inventory() {
        let mut inventory = Inventory::new(20, 100.0);

        add_to_agent_inventory(&mut inventory, ItemType::WoodenAxe, 1);
        add_to_agent_inventory(&mut inventory, ItemType::StonePickaxe, 1);

        assert_eq!(count_tools_in_inventory(&inventory), 2);
    }
}
