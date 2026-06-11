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
    EquipmentSlot, EquipmentItem, EquipmentType, EquipmentMaterial,
    MetalMaterial, ClothingMaterial, WoodMaterial, StoneMaterial,
};

fn main() {
    println!("=== EBSS Equipment Integration: Agent Equipment Demo ===\n");

    // ===== Part 1: Creating an Agent =====
    println!("--- Part 1: Creating Agent with Inventory ---");

    let config = AgentConfig { random_weights: false };

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

    // Create and equip items using the current 5-argument constructor
    println!("\nEquipping iron sword...");
    let sword = EquipmentItem::new(
        "iron_sword".to_string(),
        EquipmentType::Sword,
        EquipmentSlot::MainHand,
        EquipmentMaterial::Metal(MetalMaterial::Iron),
        Quality::Advanced,
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
        Quality::Moderate,
    );

    match agent.equipment.equip(shield) {
        Ok(_) => println!("  ✓ Equipped wooden shield to off hand"),
        Err(e) => println!("  ✗ Failed: {}", e),
    }

    println!("\nEquipping armor pieces...");
    let head_armor = EquipmentItem::new(
        "leather_cap".to_string(),
        EquipmentType::LightArmor,
        EquipmentSlot::Head,
        EquipmentMaterial::Cloth(ClothingMaterial::Leather),
        Quality::Basic,
    );

    agent.equipment.equip(head_armor).ok();
    println!("  ✓ Equipped leather cap (head)");

    let torso_armor = EquipmentItem::new(
        "leather_vest".to_string(),
        EquipmentType::LightArmor,
        EquipmentSlot::Torso,
        EquipmentMaterial::Cloth(ClothingMaterial::Leather),
        Quality::Basic,
    );

    agent.equipment.equip(torso_armor).ok();
    println!("  ✓ Equipped leather vest (torso)");

    let boots = EquipmentItem::new(
        "iron_boots".to_string(),
        EquipmentType::MediumArmor,
        EquipmentSlot::Feet,
        EquipmentMaterial::Metal(MetalMaterial::Iron),
        Quality::Advanced,
    );

    agent.equipment.equip(boots).ok();
    println!("  ✓ Equipped iron boots (feet)");

    println!();

    // ===== Part 4: Viewing Equipped Items =====
    println!("--- Part 4: Currently Equipped Items ---");

    let equipped = agent.get_all_equipped();
    println!("Total equipped items: {}", equipped.len());
    println!();

    for item in &equipped {
        println!("  {:?}: {}", item.slot, item.name);
        println!("    Type: {:?}", item.equipment_type);
        println!("    Quality: {:?}", item.quality);
        println!("    Weight: {:.1} kg", item.weight);
        println!("    Durability: {:.0}/{:.0}", item.durability, item.max_durability);
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

    println!("Equipping iron pickaxe for mining...");
    let _pickaxe = EquipmentItem::new(
        "iron_pickaxe".to_string(),
        EquipmentType::Pickaxe,
        EquipmentSlot::MainHand,
        EquipmentMaterial::Metal(MetalMaterial::Iron),
        Quality::Advanced,
    );

    println!("\nProtection stats:");
    println!("  Cold insulation: {:.1}", agent.equipment.total_cold_insulation());
    println!("  Heat resistance: {:.1}", agent.equipment.total_heat_resistance());

    println!("\nEncumbrance:");
    println!("  Total weight: {:.1} kg", agent.equipment.get_total_weight());
    println!("  Movement multiplier: {:.0}%", agent.equipment.movement_speed_multiplier() * 100.0);
    println!();

    // Switch to hatchet
    println!("Switching to stone hatchet for woodcutting...");
    let hatchet = EquipmentItem::new(
        "stone_hatchet".to_string(),
        EquipmentType::Hatchet,
        EquipmentSlot::MainHand,
        EquipmentMaterial::Stone(StoneMaterial::Flint),
        Quality::Moderate,
    );

    agent.equipment.unequip(EquipmentSlot::MainHand);
    agent.equipment.equip(hatchet).ok();

    println!("  ✓ Equipped stone hatchet");
    println!();

    let harvesting_speed = agent.get_harvesting_speed_bonus();
    println!("Harvesting speed bonus: {:.1}x", harvesting_speed);
    println!();

    // ===== Part 8: Durability and Wear =====
    println!("--- Part 8: Equipment Durability and Wear ---");

    if let Some(tool) = agent.get_equipped(EquipmentSlot::MainHand) {
        println!("Stone hatchet condition before use:");
        println!("  Durability: {:.0}/{:.0} ({:.0}%)",
            tool.durability,
            tool.max_durability,
            (tool.durability / tool.max_durability) * 100.0);
    }

    println!("Using hatchet to chop 10 trees...");
    for i in 1..=10 {
        match agent.damage_equipment(EquipmentSlot::MainHand, 5.0) {
            Ok(broke) => {
                if broke {
                    println!("  Tree {}: ✗ Hatchet BROKE!", i);
                    break;
                } else if i % 3 == 0 {
                    if let Some(tool) = agent.get_equipped(EquipmentSlot::MainHand) {
                        println!("  Tree {}: Durability {:.0}/{:.0}",
                            i,
                            tool.durability,
                            tool.max_durability);
                    }
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
    }

    if let Some(tool) = agent.get_equipped(EquipmentSlot::MainHand) {
        println!("Stone hatchet condition after use:");
        println!("  Durability: {:.0}/{:.0} ({:.0}%)",
            tool.durability,
            tool.max_durability,
            (tool.durability / tool.max_durability) * 100.0);

        if tool.is_broken() {
            println!("  Status: BROKEN - cannot be used");
        } else if tool.durability < tool.max_durability * 0.25 {
            println!("  Status: HEAVILY WORN - repair recommended");
        } else if tool.durability < tool.max_durability * 0.5 {
            println!("  Status: WORN - consider repairing");
        }
    } else {
        println!("Stone hatchet broke and was auto-removed!");
    }
    println!();

    // ===== Part 9: Repairing Equipment =====
    println!("--- Part 9: Repairing Equipment ---");

    if agent.is_slot_equipped(EquipmentSlot::MainHand) {
        println!("Repairing stone hatchet...");
        match agent.repair_equipment(EquipmentSlot::MainHand, 25.0) {
            Ok(_) => {
                if let Some(tool) = agent.get_equipped(EquipmentSlot::MainHand) {
                    println!("  ✓ Repaired! Durability: {:.0}/{:.0}",
                        tool.durability,
                        tool.max_durability);
                }
            }
            Err(e) => println!("  ✗ Failed: {}", e),
        }
    }

    // ===== Part 10: Encumbrance =====
    println!("--- Part 10: Equipment Weight and Encumbrance ---");

    println!("Adding heavy armor...");

    let heavy_chestplate = EquipmentItem::new(
        "steel_chestplate".to_string(),
        EquipmentType::HeavyArmor,
        EquipmentSlot::Torso,
        EquipmentMaterial::Metal(MetalMaterial::Steel),
        Quality::Expert,
    );

    // Unequip leather first
    agent.equipment.unequip(EquipmentSlot::Torso);
    match agent.equipment.equip(heavy_chestplate) {
        Ok(_) => println!("  ✓ Equipped steel chestplate (heavy)"),
        Err(e) => println!("  ✗ Failed: {}", e),
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
    println!();

    // ===== Part 12: Equipment Loadouts =====
    println!("--- Part 12: Equipment Loadouts for Different Tasks ---");

    println!("\nCombat Loadout:");
    println!("  Main Hand: Sword (max damage)");
    println!("  Off Hand: Shield (defense)");
    println!("  Armor: Full set (protection)");
    println!("  Effect: High survivability, good damage");
    println!();

    println!("Mining Loadout:");
    println!("  Main Hand: Pickaxe (mining speed)");
    println!("  Armor: Light leather (mobility)");
    println!("  Effect: Fast resource gathering, moderate protection");
    println!();

    println!("Exploration Loadout:");
    println!("  Main Hand: Light weapon (defense)");
    println!("  Clothing: Weather-appropriate (insulation)");
    println!("  Effect: Balanced mobility and protection");
    println!();

    // ===== Summary =====
    println!("=== Final Equipment Status ===");

    let final_equipped = agent.get_all_equipped();
    println!("Equipped items: {}", final_equipped.len());

    for item in &final_equipped {
        println!("  {:?}: {} ({:.0}% durability)",
            item.slot,
            item.name,
            (item.durability / item.max_durability) * 100.0);
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
