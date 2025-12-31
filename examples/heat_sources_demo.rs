// examples/heat_sources_demo.rs
//! Comprehensive demonstration of the heat source system
//!
//! This example shows:
//! - Different heat source types and temperatures
//! - Building heat sources in the world
//! - Fuel management and consumption
//! - Lighting and extinguishing fires
//! - Temperature zones and environmental heating
//! - Material heating and smelting simulation
//! - Heat source progression (campfire → furnace)

use ebss::world::{World, WorldConfig, ResourceConfig};
use ebss::environment::HeatSourceType;

fn main() {
    println!("=== EBSS Heat Source System Demonstration ===\n");

    // ===== Part 1: Heat Source Types and Temperatures =====
    println!("--- Part 1: Heat Source Types and Temperatures ---");

    let heat_source_types = [
        HeatSourceType::Campfire,
        HeatSourceType::BellowsFire,
        HeatSourceType::SmeltingFire,
        HeatSourceType::SmeltingPit,
        HeatSourceType::Bloomery,
        HeatSourceType::StoneFurnace,
        HeatSourceType::ClayFurnace,
        HeatSourceType::AdvancedFurnace,
    ];

    for hs_type in &heat_source_types {
        let (min, max) = hs_type.temperature_range();
        println!("{:?}:", hs_type);
        println!("  Temperature range: {:.0}°C - {:.0}°C", min, max);
        println!("  Average: {:.0}°C", hs_type.average_temperature());
        println!("  Fuel consumption: {:.2} units/tick", hs_type.fuel_consumption_rate());

        let materials = hs_type.construction_materials();
        if !materials.is_empty() {
            println!("  Materials needed:");
            for (material, count) in materials {
                println!("    - {} x{}", material, count);
            }
        }

        if let Some(tech) = hs_type.required_technology() {
            println!("  Requires technology: {}", tech);
        }
        println!();
    }

    // ===== Part 2: Melting Points Comparison =====
    println!("--- Part 2: What Can Each Heat Source Melt? ---");

    // Common melting points
    let materials = [
        ("Lead", 327.0),
        ("Tin", 232.0),
        ("Zinc", 420.0),
        ("Aluminum", 660.0),
        ("Bronze", 950.0),
        ("Copper", 1085.0),
        ("Iron", 1538.0),
        ("Steel", 1370.0),
    ];

    for hs_type in &heat_source_types {
        let (_, max_temp) = hs_type.temperature_range();
        print!("{:?} ({:.0}°C) can melt: ", hs_type, max_temp);

        let can_melt: Vec<&str> = materials
            .iter()
            .filter(|(_, mp)| *mp <= max_temp)
            .map(|(name, _)| *name)
            .collect();

        if can_melt.is_empty() {
            println!("Nothing (too cold)");
        } else {
            println!("{}", can_melt.join(", "));
        }
    }
    println!();

    // ===== Part 3: Creating a World with Heat Sources =====
    println!("--- Part 3: Building Heat Sources in World ---");

    let config = WorldConfig {
        size: (50, 50),
        initial_resources: ResourceConfig {
            stone_nodes: 100,
            wood_nodes: 50,
            ..Default::default()
        },
    };

    let mut world = World::new(config);

    println!("Created world: {} x {}", world.grid.width, world.grid.height);
    println!();

    // Build various heat sources
    println!("Building heat sources...");

    // Build a campfire
    match world.build_heat_source(HeatSourceType::Campfire, (10, 10, 0), None) {
        Ok(id) => println!("  Built Campfire at (10, 10) - ID: {}", id),
        Err(e) => println!("  Failed to build campfire: {}", e),
    }

    // Build a bloomery
    match world.build_heat_source(HeatSourceType::Bloomery, (15, 10, 0), None) {
        Ok(id) => println!("  Built Bloomery at (15, 10) - ID: {}", id),
        Err(e) => println!("  Failed to build bloomery: {}", e),
    }

    // Build a stone furnace
    match world.build_heat_source(HeatSourceType::StoneFurnace, (20, 10, 0), None) {
        Ok(id) => println!("  Built Stone Furnace at (20, 10) - ID: {}", id),
        Err(e) => println!("  Failed to build furnace: {}", e),
    }

    println!("Total heat sources: {}", world.heat_sources.all().len());
    println!();

    // ===== Part 4: Fuel Management =====
    println!("--- Part 4: Adding Fuel and Lighting Fires ---");

    // Get the campfire
    if let Some(campfire) = world.get_heat_source_at(10, 10) {
        println!("Campfire status before fueling:");
        println!("  Is lit: {}", campfire.is_lit);
        println!("  Temperature: {:.1}°C", campfire.current_temperature);
        println!("  Fuel count: {}", campfire.fuel.len());
        println!();

        let campfire_id = campfire.id;

        // Add wood fuel
        println!("Adding 50 units of wood...");
        world.add_fuel_to_heat_source(&campfire_id, "wood".to_string(), 50.0).ok();

        // Light the fire
        println!("Lighting the campfire...");
        match world.light_heat_source(&campfire_id) {
            Ok(_) => println!("  Campfire lit successfully!"),
            Err(e) => println!("  Failed to light: {}", e),
        }

        // Check status after lighting
        if let Some(campfire) = world.heat_sources.get(&campfire_id) {
            println!("\nCampfire status after lighting:");
            println!("  Is lit: {}", campfire.is_lit);
            println!("  Fuel:");
            for fuel in &campfire.fuel {
                println!("    {} - {:.1} units", fuel.material_id, fuel.amount);
            }
        }
        println!();
    }

    // ===== Part 5: Simulating Heat-Up =====
    println!("--- Part 5: Heating Up Over Time ---");

    let campfire_id = world.get_heat_source_at(10, 10).unwrap().id;

    println!("Simulating 20 ticks of heating...");
    for tick in 1..=20 {
        world.tick();

        if tick % 5 == 0 {
            if let Some(campfire) = world.heat_sources.get(&campfire_id) {
                println!("  Tick {}: Temp = {:.1}°C, Lit = {}, Fuel = {:.1}",
                    tick,
                    campfire.current_temperature,
                    campfire.is_lit,
                    campfire.fuel.first().map(|f| f.amount).unwrap_or(0.0));
            }
        }
    }
    println!();

    // ===== Part 6: Adding Materials to Heat =====
    println!("--- Part 6: Heating Materials for Smelting ---");

    let bloomery_id = world.get_heat_source_at(15, 10).unwrap().id;

    // Add fuel to bloomery
    world.add_fuel_to_heat_source(&bloomery_id, "charcoal".to_string(), 100.0).ok();
    world.light_heat_source(&bloomery_id).ok();

    // Add iron ore
    println!("Adding 10 iron ore to bloomery...");
    world.add_to_heat_source(&bloomery_id, "iron_ore".to_string(), 10).ok();

    // Add copper ore to campfire
    println!("Adding 5 copper ore to campfire...");
    world.add_to_heat_source(&campfire_id, "copper_ore".to_string(), 5).ok();

    println!();

    // Heat for a while
    println!("Heating for 30 ticks...");
    for tick in 1..=30 {
        world.tick();

        if tick % 10 == 0 {
            println!("  Tick {}:", tick);

            if let Some(bloomery) = world.heat_sources.get(&bloomery_id) {
                println!("    Bloomery: {:.0}°C", bloomery.current_temperature);
                for content in &bloomery.contents {
                    println!("      {} x{} - heated for {} ticks at {:.0}°C",
                        content.material_id,
                        content.quantity,
                        content.heating_time,
                        content.current_temp);
                }
            }

            if let Some(campfire) = world.heat_sources.get(&campfire_id) {
                println!("    Campfire: {:.0}°C", campfire.current_temperature);
                for content in &campfire.contents {
                    println!("      {} x{} - heated for {} ticks at {:.0}°C",
                        content.material_id,
                        content.quantity,
                        content.heating_time,
                        content.current_temp);
                }
            }
        }
    }
    println!();

    // ===== Part 7: Temperature Zones =====
    println!("--- Part 7: Environmental Temperature Zones ---");

    println!("Checking temperature at different positions:");

    // Near campfire
    let temp_at_campfire = world.environmental_temperature((10, 10, 0), 10.0);
    println!("  At campfire (10, 10): {:.1}°C", temp_at_campfire);

    // 3 tiles away from campfire
    let temp_near_campfire = world.environmental_temperature((13, 10, 0), 10.0);
    println!("  3 tiles from campfire (13, 10): {:.1}°C", temp_near_campfire);

    // Between campfire and bloomery
    let temp_between = world.environmental_temperature((12, 10, 0), 10.0);
    println!("  Between campfire and bloomery (12, 10): {:.1}°C", temp_between);

    // Far away
    let temp_far = world.environmental_temperature((40, 40, 0), 10.0);
    println!("  Far from all sources (40, 40): {:.1}°C", temp_far);
    println!();

    // ===== Part 8: Fuel Depletion =====
    println!("--- Part 8: Fuel Depletion and Cooling ---");

    // Create a new campfire with limited fuel
    let temp_fire_id = world.build_heat_source(HeatSourceType::Campfire, (30, 30, 0), None).unwrap();
    world.add_fuel_to_heat_source(&temp_fire_id, "wood".to_string(), 5.0).ok(); // Only 5 units
    world.light_heat_source(&temp_fire_id).ok();

    println!("Created temporary campfire with 5 units of wood");
    println!("Fuel consumption: {:.2} units/tick", HeatSourceType::Campfire.fuel_consumption_rate());
    println!("\nTracking until fuel runs out:");

    for tick in 1..=60 {
        world.tick();

        if tick % 10 == 0 {
            if let Some(fire) = world.heat_sources.get(&temp_fire_id) {
                let fuel_amount = fire.fuel.first().map(|f| f.amount).unwrap_or(0.0);
                println!("  Tick {}: Lit = {}, Temp = {:.1}°C, Fuel = {:.1}",
                    tick,
                    fire.is_lit,
                    fire.current_temperature,
                    fuel_amount);

                if !fire.is_lit && fuel_amount == 0.0 {
                    println!("    → Fire went out due to lack of fuel!");
                }
            }
        }
    }
    println!();

    // ===== Part 9: Extinguishing =====
    println!("--- Part 9: Manual Extinguishing ---");

    // The bloomery should still be burning
    if let Some(bloomery) = world.heat_sources.get(&bloomery_id) {
        println!("Bloomery before extinguishing:");
        println!("  Is lit: {}", bloomery.is_lit);
        println!("  Temperature: {:.1}°C", bloomery.current_temperature);
    }

    println!("\nExtinguishing bloomery...");
    world.extinguish_heat_source(&bloomery_id).ok();

    if let Some(bloomery) = world.heat_sources.get(&bloomery_id) {
        println!("\nBloomery after extinguishing:");
        println!("  Is lit: {}", bloomery.is_lit);
        println!("  Temperature: {:.1}°C", bloomery.current_temperature);
    }

    println!("\nCooling over 20 ticks...");
    for tick in 1..=20 {
        world.tick();

        if tick % 5 == 0 {
            if let Some(bloomery) = world.heat_sources.get(&bloomery_id) {
                println!("  Tick {}: {:.1}°C", tick, bloomery.current_temperature);
            }
        }
    }
    println!();

    // ===== Part 10: Heat Source Progression =====
    println!("--- Part 10: Technology Progression Path ---");

    println!("Typical progression for metalworking:");
    println!();

    let progression = [
        (HeatSourceType::Campfire, "Accidental smelting discovery"),
        (HeatSourceType::BellowsFire, "Intentional temperature control"),
        (HeatSourceType::SmeltingFire, "Dedicated smelting operations"),
        (HeatSourceType::SmeltingPit, "Improved pit furnaces"),
        (HeatSourceType::Bloomery, "Iron smelting capability"),
        (HeatSourceType::StoneFurnace, "Advanced stone construction"),
        (HeatSourceType::ClayFurnace, "Clay/brick for high temps"),
        (HeatSourceType::AdvancedFurnace, "Blast furnace technology"),
    ];

    for (idx, (hs_type, description)) in progression.iter().enumerate() {
        let (_, max_temp) = hs_type.temperature_range();
        println!("{}. {:?} ({:.0}°C)", idx + 1, hs_type, max_temp);
        println!("   Purpose: {}", description);

        let tech = hs_type.required_technology();
        println!("   Unlock: {}", tech.unwrap_or("No tech requirement"));
        println!();
    }

    // ===== Part 11: Summary Statistics =====
    println!("--- Part 11: World Heat Source Summary ---");

    let all_sources = world.heat_sources.all();
    let lit_sources = world.heat_sources.all_lit();

    println!("Total heat sources: {}", all_sources.len());
    println!("Currently lit: {}", lit_sources.len());
    println!();

    println!("Heat sources by type:");
    for hs_type in &heat_source_types {
        let count = all_sources.iter()
            .filter(|hs| hs.heat_source_type == *hs_type)
            .count();
        if count > 0 {
            println!("  {:?}: {}", hs_type, count);
        }
    }
    println!();

    println!("Active fires:");
    for source in &lit_sources {
        println!("  {:?} at ({}, {}, {}) - {:.0}°C",
            source.heat_source_type,
            source.position.0,
            source.position.1,
            source.position.2,
            source.current_temperature);
    }
    println!();

    // ===== Summary =====
    println!("=== Key Features Demonstrated ===");
    println!("✓ 8 heat source types from campfire to advanced furnace");
    println!("✓ Temperature ranges from 600°C to 1700°C");
    println!("✓ Material melting point compatibility");
    println!("✓ World integration with placement system");
    println!("✓ Fuel management (wood, charcoal, coal)");
    println!("✓ Fuel consumption and depletion");
    println!("✓ Lighting and extinguishing controls");
    println!("✓ Material heating and smelting preparation");
    println!("✓ Environmental temperature zones");
    println!("✓ Temperature fall-off with distance");
    println!("✓ Automatic cooling when unlit");
    println!("✓ Technology progression path");
    println!("✓ Construction material requirements");

    println!("\n=== Demonstration Complete ===");
}
