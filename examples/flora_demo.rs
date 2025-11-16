// examples/flora_demo.rs
//! Comprehensive demonstration of the flora (plant) system
//!
//! This example shows:
//! - Plant species database with 51 species
//! - Different plant categories (trees, crops, flowers, fungi, etc.)
//! - Growth stages and progression
//! - Harvesting and regrowth mechanics
//! - Biome-specific plant distributions
//! - Individual plant instances with growth tracking
//! - PlantManager for population management
//! - Farming vs wild plants
//! - Stage-specific drops

use ebss::environment::{
    PlantSpecies, FloraRegistry, ClimateZone, GrowthStage, PlantSize,
    Plant, PlantManager,
};
use uuid::Uuid;

fn main() {
    println!("=== EBSS Flora (Plant) System Demonstration ===\n");

    // ===== Part 1: Species Database =====
    println!("--- Part 1: Plant Species Database ---");

    let registry = FloraRegistry::new();
    println!("Total species registered: {}", registry.count());
    println!();

    // Show species by category
    let trees = registry.get_trees();
    println!("Trees: {} species", trees.len());
    for tree in trees.iter().take(5) {
        println!("  {} - {}", tree.name, tree.description);
    }
    println!("  ... and {} more", trees.len() - 5);
    println!();

    let crops = registry.get_crops();
    println!("Crops: {} species", crops.len());
    for crop in &crops {
        println!("  {} - growth time: {} ticks", crop.name, crop.growth_time);
    }
    println!();

    // ===== Part 2: Biome Distribution =====
    println!("--- Part 2: Biome-Specific Plants ---");

    for biome in [ClimateZone::Arctic, ClimateZone::Temperate, ClimateZone::Desert, ClimateZone::Tropical] {
        let plants = registry.get_by_biome(biome);
        println!("{:?} biome: {} species", biome, plants.len());

        // Show a few examples
        for plant in plants.iter().take(3) {
            println!("  - {}", plant.name);
        }
    }
    println!();

    // ===== Part 3: Plant Characteristics =====
    println!("--- Part 3: Plant Characteristics by Size ---");

    let all_species = registry.all_species();

    for size in [PlantSize::Tiny, PlantSize::Small, PlantSize::Medium, PlantSize::Large, PlantSize::Huge] {
        let count = all_species.iter().filter(|s| s.size == size).count();
        println!("{:?}: {} species", size, count);
    }
    println!();

    // ===== Part 4: Growth Characteristics =====
    println!("--- Part 4: Growth Times and Regrowth ---");

    // Fastest growing
    let mut by_growth = all_species.clone();
    by_growth.sort_by_key(|s| s.growth_time);

    println!("Fastest growing plants:");
    for plant in by_growth.iter().take(5) {
        println!("  {}: {} ticks", plant.name, plant.growth_time);
    }
    println!();

    println!("Slowest growing plants:");
    for plant in by_growth.iter().rev().take(5) {
        println!("  {}: {} ticks", plant.name, plant.growth_time);
    }
    println!();

    // Renewable resources
    let renewable: Vec<_> = all_species.iter().filter(|s| s.regrows).collect();
    println!("Renewable resources (regrow after harvest): {}", renewable.len());
    for plant in renewable.iter().take(5) {
        println!("  {} - regrows in {} ticks", plant.name, plant.regrow_time);
    }
    println!();

    // ===== Part 5: Tree Details =====
    println!("--- Part 5: Tree Species Details ---");

    // Basic trees
    if let Some(oak) = registry.get("oak_tree") {
        println!("Oak Tree:");
        println!("  Health: {}", oak.health);
        println!("  Growth time: {} ticks", oak.growth_time);
        println!("  Size: {:?}", oak.size);
        println!("  Wood yield: {}-{}",
            oak.drops[0].min_quantity,
            oak.drops[0].max_quantity);
    }
    println!();

    // Fruit trees
    if let Some(apple) = registry.get("apple_tree") {
        println!("Apple Tree:");
        println!("  Drops:");
        for drop in &apple.drops {
            println!("    {} x{}-{}{}",
                drop.material_id,
                drop.min_quantity,
                drop.max_quantity,
                if let Some(stage) = drop.required_stage {
                    format!(" (only at {:?} stage)", stage)
                } else {
                    String::new()
                });
        }
    }
    println!();

    // Ancient trees
    if let Some(sequoia) = registry.get("sequoia_tree") {
        println!("Sequoia Tree (ancient):");
        println!("  Health: {}", sequoia.health);
        println!("  Growth time: {} ticks ({} hours at 100 ticks/hour)",
            sequoia.growth_time,
            sequoia.growth_time / 100);
        println!("  Wood yield: {}-{}",
            sequoia.drops[0].min_quantity,
            sequoia.drops[0].max_quantity);
    }
    println!();

    // ===== Part 6: Crop Details =====
    println!("--- Part 6: Agricultural Crops ---");

    // Grains
    for grain in ["wheat", "barley", "corn", "rice"] {
        if let Some(plant) = registry.get(grain) {
            println!("{}:", plant.name);
            println!("  Growth time: {} ticks", plant.growth_time);
            println!("  Yield: {}-{} {}",
                plant.drops[0].min_quantity,
                plant.drops[0].max_quantity,
                plant.drops[0].material_id);
        }
    }
    println!();

    // Vegetables
    println!("Vegetables:");
    for veg in ["potato", "carrot", "onion", "cabbage", "tomato"] {
        if let Some(plant) = registry.get(veg) {
            let regrows = if plant.regrows { " (renewable)" } else { "" };
            println!("  {}: {} ticks{}", plant.name, plant.growth_time, regrows);
        }
    }
    println!();

    // ===== Part 7: Special Plants =====
    println!("--- Part 7: Special Plant Categories ---");

    // Medicinal plants
    println!("Medicinal/Alchemical plants:");
    for med in ["medicinal_herb", "aloe", "ginseng", "chamomile", "mandrake"] {
        if let Some(plant) = registry.get(med) {
            println!("  {} - {}", plant.name, plant.description);
        }
    }
    println!();

    // Fungi
    println!("Fungi:");
    for fungus in ["mushroom", "poisonous_mushroom", "shelf_fungus"] {
        if let Some(plant) = registry.get(fungus) {
            println!("  {} - {}", plant.name, plant.description);
        }
    }
    println!();

    // Flowers
    println!("Flowers:");
    for flower in ["rose", "lavender", "tulip", "sunflower"] {
        if let Some(plant) = registry.get(flower) {
            println!("  {} - {}", plant.name, plant.description);
        }
    }
    println!();

    // ===== Part 8: Plant Instance System =====
    println!("--- Part 8: Individual Plant Instances ---");

    let wheat_species = registry.get("wheat").unwrap();
    let mut wheat_plant = Plant::new("wheat".to_string(), (10, 20))
        .with_species(wheat_species);

    println!("Created wheat plant:");
    println!("  Position: ({}, {})", wheat_plant.position.0, wheat_plant.position.1);
    println!("  Health: {}/{}", wheat_plant.current_health, wheat_plant.max_health);
    println!("  Status: {}", wheat_plant.status());
    println!();

    // Simulate growth
    println!("Simulating growth (200 ticks)...");
    for tick in 0..200 {
        wheat_plant.grow(wheat_species);
        if tick % 50 == 0 {
            println!("  Tick {}: {}", tick, wheat_plant.status());
        }
    }

    println!("After 200 ticks:");
    println!("  Status: {}", wheat_plant.status());
    println!("  Harvestable: {}", wheat_plant.is_harvestable);
    println!();

    // Continue to maturity
    println!("Continuing growth to maturity...");
    let mut ticks = 200;
    while !wheat_plant.is_harvestable {
        wheat_plant.grow(wheat_species);
        ticks += 1;
    }
    println!("  Reached harvestable at tick {}", ticks);
    println!("  Status: {}", wheat_plant.status());
    println!();

    // Harvest
    println!("Harvesting wheat...");
    let drops = wheat_plant.harvest(wheat_species);
    println!("  Received:");
    for drop in &drops {
        println!("    {} x{}-{}", drop.material_id, drop.min_quantity, drop.max_quantity);
    }
    println!("  Status after harvest: {}", wheat_plant.status());
    println!();

    // ===== Part 9: Regrowth Demonstration =====
    println!("--- Part 9: Regrowth Mechanics ---");

    let berry_species = registry.get("berry_bush").unwrap();
    let mut berry_bush = Plant::new("berry_bush".to_string(), (15, 25))
        .with_species(berry_species);

    println!("Growing berry bush to maturity...");
    while !berry_bush.is_harvestable {
        berry_bush.grow(berry_species);
    }

    println!("  Mature! Status: {}", berry_bush.status());
    println!();

    println!("Harvesting berries...");
    let drops = berry_bush.harvest(berry_species);
    println!("  Received:");
    for drop in &drops {
        println!("    {} x{}-{}", drop.material_id, drop.min_quantity, drop.max_quantity);
    }
    println!("  Status: {}", berry_bush.status());
    println!("  Regrows: {}", berry_species.regrows);
    println!();

    println!("Waiting for regrowth...");
    for tick in 0..=berry_species.regrow_time {
        berry_bush.grow(berry_species);
        if tick % 75 == 0 {
            println!("  Tick {}: {}", tick, berry_bush.status());
        }
    }

    println!("After regrowth period:");
    println!("  Status: {}", berry_bush.status());
    println!();

    // Grow to maturity again
    println!("Growing to maturity again...");
    while !berry_bush.is_harvestable {
        berry_bush.grow(berry_species);
    }
    println!("  Status: {}", berry_bush.status());
    println!("  Can harvest again!");
    println!();

    // ===== Part 10: PlantManager System =====
    println!("--- Part 10: PlantManager Population Management ---");

    let mut manager = PlantManager::new(1000);

    // Spawn various plants
    manager.spawn_plant("oak_tree".to_string(), (0, 0));
    manager.spawn_plant("pine_tree".to_string(), (5, 5));
    manager.spawn_plant("apple_tree".to_string(), (10, 0));

    // Spawn a patch of wheat
    let wheat_ids = manager.spawn_patch("wheat".to_string(), (50, 50), 5, 0.3);
    println!("Spawned wheat patch: {} plants", wheat_ids.len());
    println!();

    // Spawn a forest
    let forest_ids = manager.spawn_patch("oak_tree".to_string(), (100, 100), 10, 0.1);
    println!("Spawned oak forest: {} trees", forest_ids.len());
    println!();

    println!("Total plants: {}", manager.total_count());
    println!();

    // ===== Part 11: Farming System =====
    println!("--- Part 11: Farming (Cultivated Plants) ---");

    let farmer_id = Uuid::new_v4();
    println!("Farmer {} planting crops...", farmer_id);

    // Plant a farm
    let mut farm_plants = Vec::new();
    for x in 0..5 {
        for y in 0..5 {
            if let Some(id) = manager.plant_crop("wheat".to_string(), (200 + x, 200 + y), farmer_id) {
                farm_plants.push(id);
            }
        }
    }

    println!("Planted {} wheat plants in 5x5 farm", farm_plants.len());
    println!();

    // Check cultivated status
    if let Some(plant) = manager.get(&farm_plants[0]) {
        println!("Sample farm plant:");
        println!("  Cultivated: {}", plant.is_cultivated);
        println!("  Planted by: {:?}", plant.planted_by);
        println!("  Status: {}", plant.status());
    }
    println!();

    // ===== Part 12: Growth Simulation =====
    println!("--- Part 12: Growth Simulation ---");

    println!("Simulating growth for 300 ticks...");
    for _ in 0..300 {
        manager.tick();
    }

    println!("After 300 ticks:");
    println!("  Total plants: {}", manager.total_count());
    println!("  Wheat count: {}", manager.count_species("wheat"));
    println!("  Harvestable wheat: {}", manager.count_harvestable("wheat"));
    println!();

    // ===== Part 13: Spatial Queries =====
    println!("--- Part 13: Spatial Queries ---");

    // Find plants near farm center
    let nearby = manager.get_in_radius((202, 202), 3.0);
    println!("Plants within radius 3 of farm center: {}", nearby.len());
    println!();

    // Find harvestable plants near farm
    let harvestable = manager.get_harvestable_in_radius((202, 202), 10.0);
    println!("Harvestable plants near farm: {}", harvestable.len());
    println!();

    // ===== Part 14: Harvesting from Manager =====
    println!("--- Part 14: Harvesting Through Manager ---");

    if !harvestable.is_empty() {
        let to_harvest = harvestable[0].id;
        println!("Harvesting plant {}...", to_harvest);

        if let Some(drops) = manager.harvest_plant(&to_harvest) {
            println!("  Received:");
            for drop in &drops {
                println!("    {} x{}-{}", drop.material_id, drop.min_quantity, drop.max_quantity);
            }
        }

        // Check status after harvest
        if let Some(plant) = manager.get(&to_harvest) {
            println!("  Status after harvest: {}", plant.status());
        }
    }
    println!();

    // ===== Part 15: Advanced Growth Stages =====
    println!("--- Part 15: Growth Stages for Fruit Trees ---");

    let orange_species = registry.get("orange_tree").unwrap();
    let mut orange = Plant::new("orange_tree".to_string(), (0, 0))
        .with_species(orange_species);

    println!("Orange tree growth stages:");
    println!("  Total growth time: {} ticks", orange_species.growth_time);
    println!();

    let stage_names = [
        "Seedling", "Growing", "Mature", "Flowering", "Fruiting"
    ];

    for (idx, stage_name) in stage_names.iter().enumerate() {
        // Grow until next stage
        let start_stage = orange.growth_stage.clone();
        while orange.growth_stage == start_stage {
            orange.grow(orange_species);
        }

        println!("  Reached {:?} stage", orange.growth_stage);

        // Check what's harvestable at this stage
        let available_drops: Vec<_> = orange_species.drops.iter()
            .filter(|d| d.required_stage.is_none() || d.required_stage == Some(orange.growth_stage))
            .collect();

        if !available_drops.is_empty() {
            println!("    Available harvests:");
            for drop in available_drops {
                println!("      {} x{}-{}", drop.material_id, drop.min_quantity, drop.max_quantity);
            }
        }
    }
    println!();

    // ===== Part 16: Plant Summary =====
    println!("--- Part 16: Summary of All Plants in World ---");

    let summaries = manager.plant_summary();
    println!("Showing first 10 plants:");
    for (idx, summary) in summaries.iter().take(10).enumerate() {
        println!("  {}. {}", idx + 1, summary);
    }
    println!("  ... and {} more plants", summaries.len().saturating_sub(10));
    println!();

    // ===== Part 17: Resource Analysis =====
    println!("--- Part 17: Resource Production Analysis ---");

    println!("Timber-producing trees:");
    let timber_trees = all_species.iter()
        .filter(|s| s.is_tree && s.drops.iter().any(|d| d.material_id == "wood"))
        .take(5);

    for tree in timber_trees {
        let wood_drop = tree.drops.iter().find(|d| d.material_id == "wood").unwrap();
        println!("  {}: {}-{} wood (growth: {} ticks)",
            tree.name,
            wood_drop.min_quantity,
            wood_drop.max_quantity,
            tree.growth_time);
    }
    println!();

    println!("Food-producing plants:");
    let food_plants = all_species.iter()
        .filter(|s| {
            s.drops.iter().any(|d|
                d.material_id.contains("fruit") ||
                d.material_id.contains("berries") ||
                d.material_id.contains("wheat") ||
                d.material_id.contains("potato")
            )
        })
        .take(5);

    for plant in food_plants {
        for drop in &plant.drops {
            if drop.material_id.contains("fruit") ||
               drop.material_id.contains("berries") ||
               drop.material_id.contains("wheat") ||
               drop.material_id.contains("potato") {
                let renewable = if plant.regrows { " (renewable)" } else { "" };
                println!("  {}: {}-{} {}{}",
                    plant.name,
                    drop.min_quantity,
                    drop.max_quantity,
                    drop.material_id,
                    renewable);
            }
        }
    }
    println!();

    // ===== Summary =====
    println!("=== Key Features Demonstrated ===");
    println!("✓ 51 plant species across 8 categories");
    println!("✓ Trees: hardwoods, softwoods, fruit trees, ancient trees");
    println!("✓ Crops: grains (wheat, barley, corn, rice) and vegetables");
    println!("✓ Flowers: roses, lavender, tulips, sunflowers");
    println!("✓ Medicinal plants: aloe, ginseng, chamomile, mandrake");
    println!("✓ Fungi: mushrooms, poisonous mushrooms, shelf fungus");
    println!("✓ Aquatic plants: reeds, lotus, seaweed");
    println!("✓ 5 growth stages: Seedling → Growing → Mature → Flowering → Fruiting");
    println!("✓ Stage-specific drops (e.g., fruits only when Fruiting)");
    println!("✓ Regrowth mechanics for renewable resources");
    println!("✓ Individual plant instances with growth tracking");
    println!("✓ PlantManager for population and farming management");
    println!("✓ Cultivated vs wild plant distinction");
    println!("✓ Spatial queries (by position, by radius)");
    println!("✓ Biome-specific distributions");
    println!("✓ Growth times ranging from 50 to 10000 ticks");

    println!("\n=== Demonstration Complete ===");
}
