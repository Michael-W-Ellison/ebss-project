// examples/minecraft_world.rs
//! Example demonstrating the Minecraft-style survival environment plugin.
//!
//! This example shows:
//! - Plugin creation and initialization
//! - Material and recipe queries
//! - Action execution
//! - World state management

use ebss::environment::*;
use ebss::core::DriveType;

fn main() {
    println!("=== EBSS Minecraft Survival Plugin Demo ===\n");

    // Create the plugin
    let mut plugin = MinecraftSurvivalPlugin::new();

    println!("📦 Plugin Created:");
    println!("   ID: {}", plugin.metadata().id);
    println!("   Name: {}", plugin.metadata().name);
    println!("   Version: {}", plugin.metadata().version);
    println!("   Author: {}", plugin.metadata().author);

    // Initialize with configuration
    let config = PluginConfig::new(12345);
    plugin.initialize(config).expect("Failed to initialize plugin");

    println!("\n✓ Plugin initialized with seed: {}", plugin.get_world_state().seed);
    println!("  World size: {:?}", (256, 256, 128));

    // Show available materials
    println!("\n--- Available Materials ({}) ---", plugin.get_materials().len());

    // Group materials by category
    let materials = plugin.get_materials();
    let mut natural: Vec<_> = materials.iter()
        .filter(|m| matches!(m.category, MaterialCategory::Natural))
        .collect();
    natural.sort_by(|a, b| a.name.cmp(&b.name));

    println!("\nNatural Resources:");
    for mat in &natural {
        println!("  {} - Hardness: {:.1}, Tool: {:?} ({:?})",
            mat.name, mat.hardness, mat.required_tool, mat.required_tier);
    }

    let tools: Vec<_> = materials.iter()
        .filter(|m| m.durability > 0)
        .collect();

    println!("\nTools (with durability):");
    for mat in &tools {
        println!("  {} - Durability: {}", mat.name, mat.durability);
    }

    let food: Vec<_> = materials.iter()
        .filter(|m| m.is_edible)
        .collect();

    println!("\nFood:");
    for mat in &food {
        println!("  {} - Food Value: {:.1}", mat.name, mat.food_value);
    }

    // Show crafting recipes
    let recipe_book = plugin.get_recipe_book();
    println!("\n--- Crafting Recipes ---");

    if let Some(planks) = recipe_book.get_recipe("planks") {
        println!("\nPlanks Recipe:");
        for input in &planks.inputs {
            println!("  Input: {} x{}", input.material_id, input.quantity);
        }
        for output in &planks.outputs {
            println!("  Output: {} x{}", output.material_id, output.quantity);
        }
    }

    if let Some(pickaxe) = recipe_book.get_recipe("wooden_pickaxe") {
        println!("\nWooden Pickaxe Recipe:");
        for input in &pickaxe.inputs {
            println!("  Input: {} x{}", input.material_id, input.quantity);
        }
        for output in &pickaxe.outputs {
            println!("  Output: {} x{}", output.material_id, output.quantity);
        }
    }

    // Execute some actions
    println!("\n--- Action Execution ---");

    // Gather wood
    let gather_action = Action::Gather { resource_type: "wood".to_string() };
    let context = ActionContext::new("demo_agent".to_string(), Position::new(5, 70, 5));

    match plugin.execute_action(&gather_action, context) {
        Ok(result) => {
            println!("\n✓ Gathered wood:");
            println!("  Success: {}", result.success);
            for item in &result.items_gained {
                println!("  + {} x{}", item.material_id, item.quantity);
            }
            println!("  Energy cost: {:.1}", result.energy_cost);
            println!("  Experience: {:.1}", result.experience);
        }
        Err(e) => println!("✗ Gather failed: {}", e),
    }

    // Craft planks
    let craft_action = Action::Craft { item_type: "planks".to_string() };
    let context = ActionContext::new("demo_agent".to_string(), Position::new(5, 70, 5));

    match plugin.execute_action(&craft_action, context) {
        Ok(result) => {
            println!("\n✓ Crafted planks:");
            println!("  Success: {}", result.success);
            for item in &result.items_consumed {
                println!("  - {} x{}", item.material_id, item.quantity);
            }
            for item in &result.items_gained {
                println!("  + {} x{}", item.material_id, item.quantity);
            }
            if let Some(drive_change) = result.drive_changes.get(&DriveType::Utility) {
                println!("  Utility drive: {:+.2}", drive_change);
            }
        }
        Err(e) => println!("✗ Craft failed: {}", e),
    }

    // Eat food
    let eat_action = Action::Eat { food_type: "apple".to_string() };
    let context = ActionContext::new("demo_agent".to_string(), Position::new(5, 70, 5));

    match plugin.execute_action(&eat_action, context) {
        Ok(result) => {
            println!("\n✓ Ate apple:");
            println!("  Success: {}", result.success);
            if let Some(drive_change) = result.drive_changes.get(&DriveType::Hunger) {
                println!("  Hunger drive: {:+.2}", drive_change);
            }
        }
        Err(e) => println!("✗ Eat failed: {}", e),
    }

    // Simulate world ticks
    println!("\n--- World Simulation ---");
    println!("Initial tick: {}", plugin.get_world_state().tick);
    println!("Initial time of day: {:.3}", plugin.get_world_state().time_of_day);

    for _ in 0..100 {
        plugin.tick();
    }

    println!("\nAfter 100 ticks:");
    println!("  Current tick: {}", plugin.get_world_state().tick);
    println!("  Time of day: {:.3}", plugin.get_world_state().time_of_day);

    // Demonstrate material lookup at position
    println!("\n--- Spatial Queries ---");

    // Check some positions
    let positions = [
        Position::new(0, 0, 0),   // Bedrock
        Position::new(0, 50, 0),  // Deep stone
        Position::new(0, 70, 0),  // Surface
    ];

    for pos in &positions {
        let mat = plugin.get_material_at(*pos);
        let walkable = plugin.is_walkable(*pos);
        println!("  Position {:?}:", (pos.x, pos.y, pos.z));
        if let Some(m) = mat {
            println!("    Material: {}", m.name);
        } else {
            println!("    Material: Air");
        }
        println!("    Walkable: {}", walkable);
    }

    // Find nearby resources
    let search_pos = Position::new(5, 50, 5);
    let nearby_coal = plugin.find_nearby_materials(search_pos, "coal", 10.0);
    println!("\n  Coal deposits within 10 blocks of {:?}: {}",
        (search_pos.x, search_pos.y, search_pos.z), nearby_coal.len());

    // Plugin registry demonstration
    println!("\n--- Plugin Registry ---");

    let mut registry = PluginRegistry::new();

    // Create a new plugin instance for the registry
    let plugin_for_registry = Box::new(MinecraftSurvivalPlugin::new());
    let config = PluginConfig::new(54321);

    match registry.register_and_activate(plugin_for_registry, config) {
        Ok(_) => {
            println!("✓ Plugin registered and activated");
            println!("  Active plugin: {:?}", registry.get_active_id());
            println!("  Total plugins: {}", registry.count());
        }
        Err(e) => println!("✗ Registration failed: {}", e),
    }

    // Summary
    println!("\n=== Summary ===");
    println!("Materials defined: {}", plugin.get_materials().len());
    println!("Actions available: {}", plugin.get_actions().len());
    println!("Recipes in book: {} (approximate)", 8); // We know we added 8 recipes

    println!("\n=== Key Features Demonstrated ===");
    println!("✓ Plugin creation and initialization");
    println!("✓ Material system with tool requirements");
    println!("✓ Crafting recipes with inputs/outputs");
    println!("✓ Action execution with results");
    println!("✓ Drive system integration");
    println!("✓ World state and tick simulation");
    println!("✓ Spatial queries (material at position)");
    println!("✓ Plugin registry for management");

    println!("\n=== Demo Complete ===");
}
