// examples/equipment_integration_demo.rs
//! Comprehensive demonstration of equipment integration with agents
//!
//! This example shows:
//! - Equipping items from inventory
//! - Unequipping items back to inventory
//! - Stat bonuses from equipment (armor, weapons, tools)
//! - Durability loss during use
//! - Encumbrance from heavy equipment
//! - Tool efficiency bonuses for tasks
//! - Combat stats calculation
//! - Environmental protection from clothing

use ebss::agents::{Agent, AgentConfig, InventoryItem, Quality};
use ebss::agents::equipment::{
    EquipmentSlot, EquipmentItem, EquipmentType,
    EquipmentMaterial, MetalMaterial, ClothingMaterial, WoodMaterial,
};

fn main() {
    println!("=== EBSS Equipment Integration: Agent Equipment Demo ===\n");

    // ===== Part 1: Creating an Agent =====
    println!("--- Part 1: Creating Agent with Inventory ---");

    let config = AgentConfig {
        random_weights: false,
        ..Default::default()
    };

    let mut agent = Agent::new(config);
    println!("Created agent: {}", agent.id);
    println!("Initial inventory items: {}", agent.inventory.get_all_items().len());
    println!();

    // ===== Part 2: Adding Equipment to Inventory =====
    println!("--- Part 2: Adding Equipment Items to Inventory ---");

    // Add various equipment items to inventory
    agent.inventory.add_item(InventoryItem::new("iron_sword".to_string(), 1));
    agent.inventory.add_item(InventoryItem::new("iron_pickaxe".to_string(), 1));
    agent.inventory.add_item(InventoryItem::new("leather_armor".to_string(), 1));
    agent.inventory.add_item(InventoryItem::new("iron_armor".to_string(), 1));
    agent.inventory.add_item(InventoryItem::new("wooden_shield".to_string(), 1));
    agent.inventory.add_item(InventoryItem::new("stone_hatchet".to_string(), 1));

    println!("Added equipment to inventory:");
    for (item_id, item) in agent.inventory.get_all_items() {
        println!("  {} x{}", item_id, item.quantity);
    }
    println!();

    // ===== Part 3: Equipping Items =====
    println!("--- Part 3: Equipping Items from Inventory ---");

    // Create and equip items using the correct API
    println!("\nEquipping iron sword...");
    let sword = EquipmentItem::new(
        "iron_sword".to_string(),
        EquipmentType::Sword,
        EquipmentSlot::MainHand,
        EquipmentMaterial::Metal(MetalMaterial::Iron),
        Quality::Moderate,
    );

    match agent.equipment.equip(sword) {
        Ok(_) => println!("  ✓ Equipped iron sword to main hand"),
        Err(e) => println!("  ✗ Failed: {}", e),
    }

    println!("\nEquipping wooden shield...");
    let shield = EquipmentItem::new(
        "wooden_shield".to_string(),
        EquipmentType::Shield,
        EquipmentSlot::OffHand,
        EquipmentMaterial::Wood(WoodMaterial::Oak),
        Quality::Basic,
    );

    match agent.equipment.equip(shield) {
        Ok(_) => println!("  ✓ Equipped wooden shield to off hand"),
        Err(e) => println!("  ✗ Failed: {}", e),
    }

    println!("\nEquipping armor pieces...");
    let leather_armor = EquipmentItem::new(
        "leather_armor".to_string(),
        EquipmentType::LightArmor,
        EquipmentSlot::Torso,
        EquipmentMaterial::Cloth(ClothingMaterial::Leather),
        Quality::Basic,
    );

    match agent.equipment.equip(leather_armor) {
        Ok(_) => println!("  ✓ Equipped leather armor to torso"),
        Err(e) => println!("  ✗ Failed: {}", e),
    }

    let iron_armor = EquipmentItem::new(
        "iron_armor".to_string(),
        EquipmentType::HeavyArmor,
        EquipmentSlot::Legs,
        EquipmentMaterial::Metal(MetalMaterial::Iron),
        Quality::Moderate,
    );

    match agent.equipment.equip(iron_armor) {
        Ok(_) => println!("  ✓ Equipped iron armor to legs"),
        Err(e) => println!("  ✗ Failed: {}", e),
    }
    println!();

    // ===== Part 4: Equipment Stats =====
    println!("--- Part 4: Equipment Stats ---");

    println!("Combat stats:");
    println!("  Weapon damage: {:.1}", agent.equipment.weapon_damage());
    println!("  Weapon speed: {:.1}", agent.equipment.weapon_attack_speed());
    println!("  Weapon range: {:.1}", agent.equipment.weapon_range());
    println!("  Total armor: {:.1}", agent.equipment.total_armor());

    println!("\nProtection stats:");
    println!("  Cold insulation: {:.1}", agent.equipment.total_cold_insulation());
    println!("  Heat resistance: {:.1}", agent.equipment.total_heat_resistance());

    println!("\nEncumbrance:");
    println!("  Total weight: {:.1} kg", agent.equipment.get_total_weight());
    println!("  Movement multiplier: {:.0}%", agent.equipment.movement_speed_multiplier() * 100.0);
    println!();

    // ===== Part 5: Tool Efficiency =====
    println!("--- Part 5: Equipping Tools ---");

    // Unequip sword to equip pickaxe
    if let Some(sword) = agent.equipment.unequip(EquipmentSlot::MainHand) {
        println!("Unequipped {} from main hand", sword.name);
    }

    let pickaxe = EquipmentItem::new(
        "iron_pickaxe".to_string(),
        EquipmentType::Pickaxe,
        EquipmentSlot::MainHand,
        EquipmentMaterial::Metal(MetalMaterial::Iron),
        Quality::Moderate,
    );

    match agent.equipment.equip(pickaxe) {
        Ok(_) => println!("  ✓ Equipped iron pickaxe"),
        Err(e) => println!("  ✗ Failed: {}", e),
    }

    if let Some(tool) = agent.equipment.get_tool_for_task("mining") {
        println!("\nMining tool stats:");
        println!("  Name: {}", tool.name);
        println!("  Mining speed: {:.1}", tool.mining_speed);
        println!("  Harvesting speed: {:.1}", tool.harvesting_speed);
        println!("  Durability: {:.1}/{:.1}", tool.durability, tool.max_durability);
    }
    println!();

    // ===== Part 6: Durability =====
    println!("--- Part 6: Durability System ---");

    // Show durability of equipped items
    println!("Equipped item durabilities:");
    for slot in [
        EquipmentSlot::MainHand,
        EquipmentSlot::OffHand,
        EquipmentSlot::Torso,
        EquipmentSlot::Legs,
    ] {
        if let Some(item) = agent.equipment.get_equipped(slot) {
            let percent = (item.durability / item.max_durability) * 100.0;
            println!("  {:?}: {} - {:.0}% ({:.1}/{:.1})",
                slot, item.name, percent, item.durability, item.max_durability);
        }
    }

    // Simulate tool use damage
    println!("\nSimulating tool use (10 uses)...");
    for _ in 0..10 {
        agent.equipment.apply_tool_wear("mining", 5.0);
    }

    if let Some(tool) = agent.equipment.get_equipped(EquipmentSlot::MainHand) {
        let percent = (tool.durability / tool.max_durability) * 100.0;
        println!("  After use: {:.0}% durability remaining", percent);
    }
    println!();

    // ===== Part 7: Equipment Summary =====
    println!("--- Part 7: Final Equipment Summary ---");

    println!("\nAll equipped items:");
    for slot in [
        EquipmentSlot::Head,
        EquipmentSlot::Torso,
        EquipmentSlot::Legs,
        EquipmentSlot::Feet,
        EquipmentSlot::MainHand,
        EquipmentSlot::OffHand,
    ] {
        if let Some(item) = agent.equipment.get_equipped(slot) {
            println!("  {:?}: {} ({:?}, {:?})",
                slot, item.name, item.equipment_type, item.quality);
        } else {
            println!("  {:?}: (empty)", slot);
        }
    }

    println!("\n=== Key Features Demonstrated ===");
    println!("✓ Equipment creation with materials and quality");
    println!("✓ Equipping items to appropriate slots");
    println!("✓ Combat stats from weapons (damage, speed, range)");
    println!("✓ Defense stats from armor (armor value, insulation)");
    println!("✓ Tool efficiency for different tasks");
    println!("✓ Durability tracking and damage");
    println!("✓ Encumbrance and movement penalty");
    println!("✓ Unequipping and re-equipping items");

    println!("\n=== Demonstration Complete ===");
}
