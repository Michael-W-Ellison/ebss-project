// examples/world_integration_demo.rs
//! Comprehensive demonstration of fauna and flora integration into the world
//!
//! This example shows:
//! - Spawning animals in the world
//! - Spawning plants in the world
//! - World tick updating both animals and plants
//! - Spatial queries for nearby animals and plants
//! - Animal interactions (taming, feeding)
//! - Plant interactions (planting, harvesting)
//! - Creating ecosystems with predator/prey dynamics
//! - Agricultural systems with crops and livestock

use ebss::world::{World, WorldConfig, ResourceConfig};
use uuid::Uuid;

fn main() {
    println!("=== EBSS World Integration: Fauna & Flora Demo ===\n");

    // ===== Part 1: World Setup =====
    println!("--- Part 1: Creating World with Integrated Systems ---");

    let config = WorldConfig {
        size: (100, 100),
        initial_resources: ResourceConfig {
            wood_nodes: 50,
            stone_nodes: 30,
            iron_nodes: 15,
            food_nodes: 40,
            ..Default::default()
        },
    };

    let mut world = World::new(config);

    println!("Created world: {} x {}", world.grid.width, world.grid.height);
    println!("Initial state:");
    println!("  Animals: {}", world.animals.population_count());
    println!("  Plants: {}", world.plants.population_count());
    println!("  Heat sources: {}", world.heat_sources.all().len());
    println!();

    // ===== Part 2: Spawning Wildlife =====
    println!("--- Part 2: Spawning Wild Animals ---");

    // Spawn various animals
    println!("Spawning wildlife...");

    // Herbivores (prey)
    let deer_group = world.spawn_animal_group("deer".to_string(), (20, 20), 8).unwrap();
    println!("  Spawned deer herd (8) at (20, 20) - Group ID: {}", deer_group);

    let rabbit_group = world.spawn_animal_group("rabbit".to_string(), (30, 25), 12).unwrap();
    println!("  Spawned rabbit colony (12) at (30, 25) - Group ID: {}", rabbit_group);

    // Predators
    let wolf_group = world.spawn_animal_group("wolf".to_string(), (50, 50), 4).unwrap();
    println!("  Spawned wolf pack (4) at (50, 50) - Group ID: {}", wolf_group);

    // Individual animals
    let bear_id = world.spawn_animal("bear".to_string(), (60, 60)).unwrap();
    println!("  Spawned bear at (60, 60) - ID: {}", bear_id);

    println!("\nTotal animals: {}", world.animals.population_count());
    println!();

    // ===== Part 3: Spawning Wild Plants =====
    println!("--- Part 3: Spawning Wild Plants ---");

    // Spawn a forest
    let oak_forest = world.spawn_plant_patch("oak_tree".to_string(), (15, 15), 10, 0.15);
    println!("  Spawned oak forest: {} trees around (15, 15)", oak_forest.len());

    let pine_forest = world.spawn_plant_patch("pine_tree".to_string(), (70, 70), 8, 0.2);
    println!("  Spawned pine forest: {} trees around (70, 70)", pine_forest.len());

    // Spawn berry bushes
    let berry_patch = world.spawn_plant_patch("berry_bush".to_string(), (40, 40), 5, 0.3);
    println!("  Spawned berry patch: {} bushes around (40, 40)", berry_patch.len());

    println!("\nTotal plants: {}", world.plants.population_count());
    println!();

    // ===== Part 4: Spatial Queries =====
    println!("--- Part 4: Spatial Queries ---");

    // Find animals near deer herd
    let nearby_animals = world.get_animals_in_radius((20, 20), 10.0);
    println!("Animals within 10 tiles of (20, 20): {}", nearby_animals.len());
    for animal in nearby_animals.iter().take(5) {
        println!("  {} at ({}, {}) - {:?}",
            animal.species_id,
            animal.position.0,
            animal.position.1,
            animal.state);
    }
    println!();

    // Find plants near oak forest
    let nearby_plants = world.get_plants_in_radius((15, 15), 15.0);
    println!("Plants within 15 tiles of (15, 15): {}", nearby_plants.len());
    for plant in nearby_plants.iter().take(5) {
        println!("  {} at ({}, {}) - {}",
            plant.species_id,
            plant.position.0,
            plant.position.1,
            plant.status());
    }
    println!();

    // ===== Part 5: Creating a Farm =====
    println!("--- Part 5: Agricultural System - Creating a Farm ---");

    let farmer_id = Uuid::new_v4();
    println!("Farmer {} establishing farm at (50, 10)...", farmer_id);

    // Plant crops in a 10x10 grid
    let mut farm_plants = Vec::new();
    println!("Planting wheat field...");

    for x in 45..55 {
        for y in 5..15 {
            if let Ok(plant_id) = world.plant_crop("wheat".to_string(), (x, y), farmer_id) {
                farm_plants.push(plant_id);
            }
        }
    }

    println!("  Planted {} wheat plants", farm_plants.len());
    println!();

    // Plant vegetables
    println!("Planting vegetable garden...");
    let mut vegetable_count = 0;

    for x in 56..61 {
        for y in 5..15 {
            let crop = match (x + y) % 4 {
                0 => "carrot",
                1 => "potato",
                2 => "onion",
                _ => "cabbage",
            };

            if world.plant_crop(crop.to_string(), (x, y), farmer_id).is_ok() {
                vegetable_count += 1;
            }
        }
    }

    println!("  Planted {} vegetable plants", vegetable_count);
    println!();

    // Check cultivated plants
    let cultivated = world.get_cultivated_plants();
    println!("Total cultivated plants: {}", cultivated.len());
    println!();

    // ===== Part 6: Domesticating Animals =====
    println!("--- Part 6: Domesticating Animals for Livestock ---");

    // Spawn animals to domesticate
    let chicken_id = world.spawn_animal("chicken".to_string(), (50, 20)).unwrap();
    let cow_id = world.spawn_animal("cow".to_string(), (52, 20)).unwrap();
    let sheep_id = world.spawn_animal("sheep".to_string(), (54, 20)).unwrap();

    println!("Spawned farm animals:");
    println!("  Chicken at (50, 20)");
    println!("  Cow at (52, 20)");
    println!("  Sheep at (54, 20)");
    println!();

    // Tame them over multiple sessions
    println!("Taming animals (10 sessions)...");
    for session in 1..=10 {
        world.tame_animal(&chicken_id, 0.1).ok();
        world.tame_animal(&cow_id, 0.1).ok();
        world.tame_animal(&sheep_id, 0.1).ok();

        if session % 3 == 0 {
            let chicken = world.animals.get(&chicken_id).unwrap();
            println!("  Session {}: Chicken tame level = {:.0}%, domesticated = {}",
                session,
                chicken.tame_level * 100.0,
                chicken.is_domesticated);
        }
    }
    println!();

    // Check domesticated animals
    let domesticated = world.get_domesticated_animals();
    println!("Domesticated animals: {}", domesticated.len());
    for animal in &domesticated {
        println!("  {} - tame level: {:.0}%", animal.species_id, animal.tame_level * 100.0);
    }
    println!();

    // ===== Part 7: World Tick Simulation =====
    println!("--- Part 7: Simulating World Over Time ---");

    println!("Running 100 ticks...");
    for tick in 1..=100 {
        world.tick();

        if tick % 25 == 0 {
            println!("\n  Tick {}:", tick);
            println!("    Animals: {}", world.animals.population_count());
            println!("    Plants: {}", world.plants.population_count());

            // Check plant growth
            let harvestable = world.get_harvestable_plants((50, 10), 20.0);
            println!("    Harvestable plants near farm: {}", harvestable.len());

            // Sample some animals
            let all_animals = world.animals.all_animals();
            if let Some(sample) = all_animals.first() {
                println!("    Sample animal: {} - age: {} ticks, state: {:?}",
                    sample.species_id,
                    sample.age,
                    sample.state);
            }

            // Sample some plants
            let all_plants = world.plants.all_plants();
            if let Some(sample) = all_plants.first() {
                println!("    Sample plant: {} - {}",
                    sample.species_id,
                    sample.status());
            }
        }
    }
    println!();

    // ===== Part 8: Harvesting Crops =====
    println!("--- Part 8: Harvesting Mature Crops ---");

    let harvestable = world.get_harvestable_plants((50, 10), 25.0);
    println!("Harvestable plants near farm: {}", harvestable.len());

    if !harvestable.is_empty() {
        println!("\nHarvesting first 5 plants...");
        for plant in harvestable.iter().take(5) {
            let plant_id = plant.id;

            match world.harvest_plant(&plant_id) {
                Ok(drops) => {
                    println!("  Harvested {}:", plant.species_id);
                    for drop in drops {
                        println!("    - {} x{}-{}",
                            drop.material_id,
                            drop.min_quantity,
                            drop.max_quantity);
                    }
                }
                Err(e) => println!("  Failed to harvest: {}", e),
            }
        }
    }
    println!();

    // ===== Part 9: Animal Interactions =====
    println!("--- Part 9: Animal Feeding and Care ---");

    // Feed domesticated animals
    println!("Feeding livestock...");
    for animal in domesticated.iter() {
        world.feed_animal(&animal.id, 10.0).ok();
        println!("  Fed {} (stamina boost)", animal.species_id);
    }
    println!();

    // ===== Part 10: Ecosystem Statistics =====
    println!("--- Part 10: Ecosystem Statistics ---");

    println!("\nAnimal Population by Species:");
    let species_list = ["deer", "rabbit", "wolf", "bear", "chicken", "cow", "sheep"];
    for species in &species_list {
        let animals = world.get_animals_by_species(species);
        if !animals.is_empty() {
            println!("  {}: {}", species, animals.len());
        }
    }
    println!();

    println!("Plant Population by Type:");
    let plant_species = ["oak_tree", "pine_tree", "berry_bush", "wheat", "carrot", "potato"];
    for species in &plant_species {
        let plants = world.get_plants_by_species(species);
        if !plants.is_empty() {
            println!("  {}: {}", species, plants.len());
        }
    }
    println!();

    println!("Domestication Status:");
    println!("  Wild animals: {}", world.animals.all_animals().len() - domesticated.len());
    println!("  Domesticated animals: {}", domesticated.len());
    println!();

    println!("Agriculture Status:");
    let wild_plants = world.plants.population_count() - cultivated.len();
    println!("  Wild plants: {}", wild_plants);
    println!("  Cultivated plants: {}", cultivated.len());
    println!();

    // ===== Part 11: Advanced Spatial Queries =====
    println!("--- Part 11: Advanced Ecosystem Queries ---");

    // Find predators near herbivores
    let deer_pos = (20, 20);
    let nearby_predators: Vec<_> = world.get_animals_in_radius(deer_pos, 20.0)
        .into_iter()
        .filter(|a| a.species_id == "wolf" || a.species_id == "bear")
        .collect();

    println!("Predators within 20 tiles of deer herd: {}", nearby_predators.len());
    for predator in &nearby_predators {
        let distance = ((predator.position.0 - deer_pos.0).pow(2) +
                        (predator.position.1 - deer_pos.1).pow(2)) as f32;
        let distance = distance.sqrt();
        println!("  {} at distance {:.1}", predator.species_id, distance);
    }
    println!();

    // Find food sources for herbivores
    let food_plants = world.get_plants_in_radius(deer_pos, 15.0);
    println!("Food plants within 15 tiles of deer: {}", food_plants.len());
    println!();

    // ===== Summary =====
    println!("=== Final World State ===");
    println!("Total Animals: {}", world.animals.population_count());
    println!("  - Wild: {}", world.animals.population_count() - domesticated.len());
    println!("  - Domesticated: {}", world.get_domesticated_animals().len());
    println!();
    println!("Total Plants: {}", world.plants.population_count());
    println!("  - Wild: {}", world.plants.population_count() - world.get_cultivated_plants().len());
    println!("  - Cultivated: {}", world.get_cultivated_plants().len());
    println!();
    println!("Buildings: {}", world.buildings.len());
    println!("Heat Sources: {}", world.heat_sources.all().len());
    println!("Resources: {}", world.resources.len());
    println!("World Tick: {}", world.tick);

    println!("\n=== Key Features Demonstrated ===");
    println!("✓ AnimalManager integrated into World");
    println!("✓ PlantManager integrated into World");
    println!("✓ spawn_animal() and spawn_animal_group()");
    println!("✓ spawn_plant() and plant_crop()");
    println!("✓ spawn_plant_patch() for forests/fields");
    println!("✓ World tick updating animals and plants");
    println!("✓ Spatial queries (get_in_radius, get_at_position)");
    println!("✓ Animal taming and domestication");
    println!("✓ Animal feeding");
    println!("✓ Plant harvesting with drops");
    println!("✓ Cultivated vs wild distinction");
    println!("✓ Species filtering");
    println!("✓ Agricultural systems (farms with crops)");
    println!("✓ Livestock management");
    println!("✓ Ecosystem creation (predators, prey, plants)");

    println!("\n=== Demonstration Complete ===");
}
