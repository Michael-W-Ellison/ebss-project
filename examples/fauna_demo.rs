// examples/fauna_demo.rs
//! Comprehensive demonstration of the fauna (animal) system
//!
//! This example shows:
//! - 35 distinct animal species across all biomes
//! - Animal behaviors (passive, aggressive, territorial, etc.)
//! - Individual animal instances with AI states
//! - Herd/pack spawning and group behavior
//! - Domestication and taming mechanics
//! - Animal products (milk, wool, eggs)
//! - Animal drops when hunted
//! - Animal manager population control
//! - Age, maturity, and reproduction

use ebss::environment::{
    FaunaRegistry, Animal, AnimalManager, AnimalBehavior, AnimalState,
    DietType, AnimalSize, ClimateZone,
};

fn main() {
    println!("=== EBSS Fauna (Animal) System Demonstration ===\n");

    // ===== Part 1: Animal Species Registry =====
    println!("--- Part 1: Animal Species Database ---");

    let registry = FaunaRegistry::new();
    let all_species = registry.all_species();

    println!("Total species: {}", all_species.len());
    println!();

    // ===== Part 2: Animals by Biome =====
    println!("--- Part 2: Animals by Biome ---");

    let biomes = [
        ClimateZone::Temperate,
        ClimateZone::Arctic,
        ClimateZone::Desert,
        ClimateZone::Tropical,
    ];

    for biome in biomes {
        let animals = registry.get_by_biome(biome);
        println!("{:?}: {} species", biome, animals.len());
        for animal in animals.iter().take(5) {
            println!("  - {} ({})", animal.name, animal.id);
        }
        if animals.len() > 5 {
            println!("  ... and {} more", animals.len() - 5);
        }
        println!();
    }

    // ===== Part 3: Animals by Behavior =====
    println!("--- Part 3: Animals by Behavior ---");

    let behaviors = [
        AnimalBehavior::Passive,
        AnimalBehavior::Neutral,
        AnimalBehavior::Defensive,
        AnimalBehavior::Aggressive,
        AnimalBehavior::Territorial,
    ];

    for behavior in behaviors {
        let animals = registry.get_by_behavior(behavior);
        println!("{:?}: {} species", behavior, animals.len());
        for animal in animals.iter().take(3) {
            println!("  - {}", animal.name);
        }
        println!();
    }

    // ===== Part 4: Detailed Species Examples =====
    println!("--- Part 4: Detailed Species Information ---");

    let examples = ["rabbit", "wolf", "bear", "sheep", "chicken", "mammoth"];

    for species_id in examples {
        if let Some(species) = registry.get(species_id) {
            println!("{} ({:?}):", species.name, species.size);
            println!("  {}", species.description);
            println!("  Health: {:.0}, Attack: {:.0}, Defense: {:.0}, Speed: {:.1}x",
                species.health, species.attack_damage, species.defense, species.speed);
            println!("  Behavior: {:?}, Diet: {:?}", species.behavior, species.diet);
            println!("  Group size: {}-{}", species.group_size.0, species.group_size.1);
            println!("  Domesticable: {}", species.can_domesticate);

            if !species.drops.is_empty() {
                println!("  Drops:");
                for drop in &species.drops {
                    println!("    - {}: {}-{} ({}% chance)",
                        drop.material_id,
                        drop.min_quantity,
                        drop.max_quantity,
                        (drop.drop_chance * 100.0) as u32);
                }
            }

            if !species.living_products.is_empty() {
                println!("  Living products:");
                for product in &species.living_products {
                    println!("    - {} x{} every {} ticks",
                        product.material_id,
                        product.quantity,
                        product.production_time);
                }
            }
            println!();
        }
    }

    // ===== Part 5: Size Categories =====
    println!("--- Part 5: Size Categories ---");

    let sizes = [
        AnimalSize::Tiny,
        AnimalSize::Small,
        AnimalSize::Medium,
        AnimalSize::Large,
        AnimalSize::Huge,
    ];

    for size in sizes {
        let count = all_species.iter().filter(|s| s.size == size).count();
        println!("{:?}: {} species", size, count);
        let examples: Vec<_> = all_species.iter()
            .filter(|s| s.size == size)
            .take(3)
            .map(|s| &s.name)
            .collect();
        println!("  Examples: {}", examples.join(", "));
        println!();
    }

    // ===== Part 6: Diet Types =====
    println!("--- Part 6: Diet Distribution ---");

    let diets = [DietType::Herbivore, DietType::Carnivore, DietType::Omnivore];

    for diet in diets {
        let count = all_species.iter().filter(|s| s.diet == diet).count();
        println!("{:?}: {} species ({:.0}%)",
            diet,
            count,
            (count as f32 / all_species.len() as f32 * 100.0));
    }
    println!();

    // ===== Part 7: Domesticable Animals =====
    println!("--- Part 7: Domesticable Animals ---");

    let domesticable = registry.get_domesticable();
    println!("Total domesticable species: {}", domesticable.len());
    println!();

    for animal in &domesticable {
        println!("{}:", animal.name);
        println!("  {}", animal.description);
        if !animal.living_products.is_empty() {
            println!("  Products: {}",
                animal.living_products.iter()
                    .map(|p| &p.material_id)
                    .collect::<Vec<_>>()
                    .join(", "));
        }
        println!();
    }

    // ===== Part 8: Individual Animal Instances =====
    println!("--- Part 8: Individual Animal Instances ---");

    let sheep_species = registry.get("sheep").unwrap();
    let mut sheep1 = Animal::new("sheep".to_string(), (10, 10), sheep_species);
    let mut sheep2 = Animal::new("sheep".to_string(), (11, 10), sheep_species);

    println!("Created 2 sheep at positions:");
    println!("  Sheep 1: {:?}, Health: {:.0}/{:.0}, Age: {}",
        sheep1.position, sheep1.current_health, sheep1.max_health, sheep1.age);
    println!("  Sheep 2: {:?}, Health: {:.0}/{:.0}, Age: {}",
        sheep2.position, sheep2.current_health, sheep2.max_health, sheep2.age);
    println!();

    // ===== Part 9: Animal States and Behavior =====
    println!("--- Part 9: Animal AI States ---");

    sheep1.state = AnimalState::Grazing;
    sheep2.state = AnimalState::Resting;

    println!("Sheep states:");
    println!("  Sheep 1: {:?}", sheep1.state);
    println!("  Sheep 2: {:?}", sheep2.state);
    println!();

    // Simulate aging and stamina
    println!("Simulating 50 ticks...");
    for _ in 0..50 {
        sheep1.tick_age();
        sheep2.tick_age();
        sheep2.recover_stamina(1.0);
    }

    println!("After 50 ticks:");
    println!("  Sheep 1: Age {}, Stamina {:.0}%",
        sheep1.age,
        sheep1.stamina_percentage() * 100.0);
    println!("  Sheep 2: Age {}, Stamina {:.0}% (resting)",
        sheep2.age,
        sheep2.stamina_percentage() * 100.0);
    println!();

    // ===== Part 10: Combat and Damage =====
    println!("--- Part 10: Combat and Damage ---");

    let wolf_species = registry.get("wolf").unwrap();
    let mut wolf = Animal::new("wolf".to_string(), (12, 10), wolf_species);

    println!("Wolf attacks Sheep 1!");
    println!("  Wolf: Attack {:.0}", wolf_species.attack_damage);
    println!("  Sheep: Defense {:.0}", sheep_species.defense);

    let damage = wolf_species.attack_damage - sheep_species.defense;
    sheep1.take_damage(damage);

    println!("  Sheep takes {:.0} damage!", damage);
    println!("  Sheep health: {:.0}/{:.0} ({:.0}%)",
        sheep1.current_health,
        sheep1.max_health,
        sheep1.health_percentage() * 100.0);
    println!();

    // ===== Part 11: Domestication and Taming =====
    println!("--- Part 11: Domestication System ---");

    let mut wild_sheep = Animal::new("sheep".to_string(), (20, 20), sheep_species);

    println!("Wild sheep:");
    println!("  Is wild: {}", wild_sheep.is_wild());
    println!("  Tame level: {:.0}%", wild_sheep.tame_level * 100.0);
    println!();

    println!("Taming the sheep (10 sessions)...");
    for session in 1..=10 {
        wild_sheep.tame(0.1); // 10% per session
        if session % 3 == 0 {
            println!("  Session {}: Tame {:.0}%, Domesticated: {}",
                session,
                wild_sheep.tame_level * 100.0,
                wild_sheep.is_domesticated);
        }
    }

    println!("\nFinal state:");
    println!("  Is wild: {}", wild_sheep.is_wild());
    println!("  Is domesticated: {}", wild_sheep.is_domesticated);
    println!();

    // ===== Part 12: Animal Manager =====
    println!("--- Part 12: Animal Manager and Population ---");

    let mut manager = AnimalManager::new(100);

    println!("Spawning animals...");

    // Spawn some rabbits
    for i in 0..5 {
        manager.spawn_animal("rabbit".to_string(), (i, 0));
    }

    // Spawn a wolf pack
    manager.spawn_group("wolf".to_string(), (10, 10), 5);

    // Spawn some deer
    manager.spawn_group("deer".to_string(), (20, 20), 8);

    // Spawn individual animals
    manager.spawn_animal("bear".to_string(), (5, 5));
    manager.spawn_animal("fox".to_string(), (7, 7));

    println!("Population:");
    println!("  Total animals: {}", manager.population_count());
    println!("  Rabbits: {}", manager.count_by_species("rabbit"));
    println!("  Wolves: {}", manager.count_by_species("wolf"));
    println!("  Deer: {}", manager.count_by_species("deer"));
    println!();

    // ===== Part 13: Spatial Queries =====
    println!("--- Part 13: Spatial Queries ---");

    let at_origin = manager.get_at_position((0, 0));
    println!("Animals at (0, 0): {}", at_origin.len());

    let near_wolves = manager.get_in_radius((10, 10), 5.0);
    println!("Animals within 5 units of (10, 10): {}", near_wolves.len());
    for animal in near_wolves {
        let species = registry.get(&animal.species_id).unwrap();
        println!("  - {} at {:?}", species.name, animal.position);
    }
    println!();

    // ===== Part 14: Aggressive Animals =====
    println!("--- Part 14: Aggressive Animals ---");

    let aggressive_count = manager.count_by_behavior(AnimalBehavior::Aggressive);
    println!("Aggressive animals: {}", aggressive_count);

    let passive_count = manager.count_by_behavior(AnimalBehavior::Passive);
    println!("Passive animals: {}", passive_count);
    println!();

    // ===== Part 15: Animal Aging and Maturity =====
    println!("--- Part 15: Aging and Maturity ---");

    let chicken_species = registry.get("chicken").unwrap();
    let mut chicken = Animal::new("chicken".to_string(), (30, 30), chicken_species);

    println!("Young chicken:");
    println!("  Age: {}, Maturity age: {}", chicken.age, chicken.maturity_age);
    println!("  Is mature: {}", chicken.is_mature());
    println!();

    println!("Aging to maturity...");
    while !chicken.is_mature() {
        chicken.tick_age();
    }

    println!("Mature chicken:");
    println!("  Age: {}", chicken.age);
    println!("  Is mature: {}", chicken.is_mature());
    println!("  Can reproduce: {}", chicken.can_reproduce);
    println!();

    // ===== Part 16: Living Products =====
    println!("--- Part 16: Living Products (Milk, Wool, Eggs) ---");

    println!("Animals that produce resources while alive:");
    for species in &domesticable {
        if !species.living_products.is_empty() {
            println!("\n{}:", species.name);
            for product in &species.living_products {
                println!("  {} x{} every {} ticks",
                    product.material_id,
                    product.quantity,
                    product.production_time);
            }
        }
    }
    println!();

    // ===== Part 17: Predator vs Prey Analysis =====
    println!("--- Part 17: Predator vs Prey Analysis ---");

    let mut predators = Vec::new();
    let mut prey = Vec::new();

    for species in all_species {
        match species.diet {
            DietType::Carnivore => {
                if species.size == AnimalSize::Small || species.size == AnimalSize::Large {
                    predators.push(&species.name);
                }
            }
            DietType::Herbivore => {
                if species.size == AnimalSize::Tiny || species.size == AnimalSize::Medium {
                    prey.push(&species.name);
                }
            }
            _ => {}
        }
    }

    println!("Predators ({}):", predators.len());
    for name in predators.iter().take(5) {
        println!("  - {}", name);
    }

    println!("\nPrey ({}):", prey.len());
    for name in prey.iter().take(5) {
        println!("  - {}", name);
    }
    println!();

    // ===== Part 18: Biome Specialists =====
    println!("--- Part 18: Biome Specialists ---");

    println!("Arctic specialists:");
    let arctic = registry.get_by_biome(ClimateZone::Arctic);
    for animal in arctic.iter().filter(|a| a.primary_biomes.contains(&ClimateZone::Arctic)) {
        println!("  - {}: {}", animal.name, animal.description);
    }
    println!();

    println!("Tropical specialists:");
    let tropical = registry.get_by_biome(ClimateZone::Tropical);
    for animal in tropical.iter().filter(|a| a.primary_biomes.contains(&ClimateZone::Tropical)).take(3) {
        println!("  - {}: {}", animal.name, animal.description);
    }
    println!();

    // ===== Part 19: Manager Tick Simulation =====
    println!("--- Part 19: Population Simulation (100 ticks) ---");

    println!("Initial population: {}", manager.population_count());

    // Simulate 100 ticks
    for _ in 0..100 {
        manager.tick();
    }

    println!("After 100 ticks:");
    println!("  Population: {}", manager.population_count());

    // Check oldest animal
    let oldest = manager.get_all().iter()
        .filter(|a| a.is_alive())
        .max_by_key(|a| a.age);

    if let Some(animal) = oldest {
        let species = registry.get(&animal.species_id).unwrap();
        println!("  Oldest animal: {} (age {})", species.name, animal.age);
    }
    println!();

    // ===== Part 20: Dangerous Animals Warning =====
    println!("--- Part 20: Most Dangerous Animals ---");

    let mut dangerous: Vec<_> = all_species.iter()
        .filter(|s| s.behavior == AnimalBehavior::Aggressive || s.behavior == AnimalBehavior::Territorial)
        .collect();

    dangerous.sort_by(|a, b| b.attack_damage.partial_cmp(&a.attack_damage).unwrap());

    println!("Top 5 most dangerous animals:");
    for (i, species) in dangerous.iter().take(5).enumerate() {
        println!("  {}. {} - Attack: {:.0}, Health: {:.0}, Defense: {:.0}",
            i + 1,
            species.name,
            species.attack_damage,
            species.health,
            species.defense);
    }
    println!();

    // ===== Summary =====
    println!("=== Key Features Demonstrated ===");
    println!("✓ 35 distinct animal species");
    println!("✓ 5 animal behaviors (Passive, Neutral, Defensive, Aggressive, Territorial)");
    println!("✓ 3 diet types (Herbivore, Carnivore, Omnivore)");
    println!("✓ 5 size categories (Tiny to Huge)");
    println!("✓ Biome-specific distributions (Arctic, Desert, Tropical, Temperate)");
    println!("✓ {} domesticable species", domesticable.len());
    println!("✓ Individual animal instances with position, health, stamina");
    println!("✓ 9 AI states (Idle, Grazing, Hunting, Fleeing, etc.)");
    println!("✓ Herd/pack spawning and group affiliation");
    println!("✓ Taming and domestication mechanics");
    println!("✓ Age, maturity, and reproduction systems");
    println!("✓ Living products (milk, wool, eggs)");
    println!("✓ Animal drops when hunted (meat, fur, leather, special items)");
    println!("✓ Animal manager with population control");
    println!("✓ Spatial queries (position, radius)");
    println!("✓ Combat and damage system");

    println!("\n=== Demonstration Complete ===");
}
