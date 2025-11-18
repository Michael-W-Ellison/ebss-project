// src/environment/fauna.rs
//! Animal life and wildlife system with biome distributions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::flora::ClimateZone;
use uuid::Uuid;

/// Animal behavior classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimalBehavior {
    Passive,    // Flees from threats
    Neutral,    // Ignores unless provoked
    Defensive,  // Attacks when cornered
    Aggressive, // Attacks on sight
    Territorial, // Attacks near den/territory
}

/// Animal diet type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DietType {
    Herbivore,
    Carnivore,
    Omnivore,
}

/// Size classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimalSize {
    Tiny,      // Rabbits, squirrels
    Small,     // Foxes, wolves
    Medium,    // Deer, sheep
    Large,     // Bears, cattle
    Huge,      // Mammoths, elephants
}

/// An animal species
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimalSpecies {
    pub id: String,
    pub name: String,
    pub description: String,

    /// Combat/health stats
    pub health: f32,
    pub attack_damage: f32,
    pub defense: f32,
    pub speed: f32, // Movement speed multiplier

    /// Behavior
    pub behavior: AnimalBehavior,
    pub diet: DietType,
    pub size: AnimalSize,

    /// Habitat
    pub primary_biomes: Vec<ClimateZone>,
    pub secondary_biomes: Vec<ClimateZone>,
    pub group_size: (u32, u32), // Min, max herd/pack size

    /// Drops when hunted/killed
    pub drops: Vec<AnimalDrop>,

    /// Whether this animal can be domesticated
    pub can_domesticate: bool,
    /// Products from living animal (milk, wool, eggs)
    pub living_products: Vec<AnimalProduct>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimalDrop {
    pub material_id: String,
    pub min_quantity: u32,
    pub max_quantity: u32,
    pub drop_chance: f32, // 0.0 to 1.0
}

impl AnimalDrop {
    pub fn new(material_id: String, min: u32, max: u32) -> Self {
        Self {
            material_id,
            min_quantity: min,
            max_quantity: max,
            drop_chance: 1.0,
        }
    }

    pub fn with_chance(mut self, chance: f32) -> Self {
        self.drop_chance = chance.clamp(0.0, 1.0);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimalProduct {
    pub material_id: String,
    pub production_time: u32, // Ticks between production
    pub quantity: u32,
}

/// Animal species database
#[derive(Debug, Clone)]
pub struct FaunaRegistry {
    species: HashMap<String, AnimalSpecies>,
}

impl FaunaRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            species: HashMap::new(),
        };

        registry.register_all_species();
        registry
    }

    fn register(&mut self, species: AnimalSpecies) {
        self.species.insert(species.id.clone(), species);
    }

    pub fn get(&self, id: &str) -> Option<&AnimalSpecies> {
        self.species.get(id)
    }

    pub fn get_by_biome(&self, biome: ClimateZone) -> Vec<&AnimalSpecies> {
        self.species
            .values()
            .filter(|s| s.primary_biomes.contains(&biome) || s.secondary_biomes.contains(&biome))
            .collect()
    }

    pub fn get_by_behavior(&self, behavior: AnimalBehavior) -> Vec<&AnimalSpecies> {
        self.species
            .values()
            .filter(|s| s.behavior == behavior)
            .collect()
    }

    fn register_all_species(&mut self) {
        // Tiny passive animals
        self.register(rabbit());
        self.register(squirrel());
        self.register(chicken());
        self.register(duck());
        self.register(goose());

        // Birds
        self.register(crow());
        self.register(eagle());
        self.register(hawk());
        self.register(owl());
        self.register(parrot());

        // Small animals
        self.register(fox());
        self.register(wolf());
        self.register(snake());

        // Medium herbivores (domesticable)
        self.register(deer());
        self.register(sheep());
        self.register(goat());
        self.register(elk_animal());
        self.register(reindeer_animal());

        // Medium/Large omnivores
        self.register(boar());
        self.register(pig());
        self.register(cow());

        // Large predators
        self.register(bear());
        self.register(lion());
        self.register(tiger());
        self.register(crocodile());

        // Arctic/Desert/Tropical specialists
        self.register(arctic_fox());
        self.register(polar_bear());
        self.register(camel());
        self.register(mammoth());
        self.register(monkey());

        // Aquatic
        self.register(fish());
        self.register(otter());
        self.register(seal());
    }

    pub fn all_species(&self) -> Vec<&AnimalSpecies> {
        self.species.values().collect()
    }

    pub fn get_domesticable(&self) -> Vec<&AnimalSpecies> {
        self.species
            .values()
            .filter(|s| s.can_domesticate)
            .collect()
    }
}

// ============================================================================
// TINY PASSIVE ANIMALS
// ============================================================================

fn rabbit() -> AnimalSpecies {
    AnimalSpecies {
        id: "rabbit".to_string(),
        name: "Rabbit".to_string(),
        description: "Small, quick herbivore, common in grasslands".to_string(),
        health: 15.0,
        attack_damage: 1.0,
        defense: 0.0,
        speed: 1.5,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        group_size: (1, 3),
        drops: vec![
            AnimalDrop::new("rabbit_meat".to_string(), 1, 2),
            AnimalDrop::new("fur".to_string(), 1, 1),
            AnimalDrop::new("leather".to_string(), 1, 1).with_chance(0.7),
        ],
        can_domesticate: true,
        living_products: vec![],
    }
}

fn squirrel() -> AnimalSpecies {
    AnimalSpecies {
        id: "squirrel".to_string(),
        name: "Squirrel".to_string(),
        description: "Nimble tree-dweller, stores nuts for winter".to_string(),
        health: 10.0,
        attack_damage: 0.5,
        defense: 0.0,
        speed: 1.8,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("squirrel_meat".to_string(), 1, 1),
            AnimalDrop::new("fur".to_string(), 1, 1),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn chicken() -> AnimalSpecies {
    AnimalSpecies {
        id: "chicken".to_string(),
        name: "Chicken".to_string(),
        description: "Common fowl, easily domesticated for eggs and meat".to_string(),
        health: 12.0,
        attack_damage: 0.5,
        defense: 0.0,
        speed: 1.2,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Omnivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        group_size: (3, 8),
        drops: vec![
            AnimalDrop::new("chicken_meat".to_string(), 2, 3),
            AnimalDrop::new("feathers".to_string(), 3, 5),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "egg".to_string(),
                production_time: 100,
                quantity: 1,
            },
        ],
    }
}

// ============================================================================
// SMALL ANIMALS
// ============================================================================

fn fox() -> AnimalSpecies {
    AnimalSpecies {
        id: "fox".to_string(),
        name: "Fox".to_string(),
        description: "Cunning predator, hunts small game".to_string(),
        health: 30.0,
        attack_damage: 8.0,
        defense: 2.0,
        speed: 1.6,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (1, 2),
        drops: vec![
            AnimalDrop::new("fox_meat".to_string(), 2, 3),
            AnimalDrop::new("fur".to_string(), 2, 3),
            AnimalDrop::new("leather".to_string(), 1, 2),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn wolf() -> AnimalSpecies {
    AnimalSpecies {
        id: "wolf".to_string(),
        name: "Wolf".to_string(),
        description: "Pack hunter, dangerous in groups".to_string(),
        health: 45.0,
        attack_damage: 12.0,
        defense: 3.0,
        speed: 1.7,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (3, 7),
        drops: vec![
            AnimalDrop::new("wolf_meat".to_string(), 3, 5),
            AnimalDrop::new("fur".to_string(), 3, 4),
            AnimalDrop::new("leather".to_string(), 2, 3),
            AnimalDrop::new("wolf_fang".to_string(), 1, 2).with_chance(0.8),
        ],
        can_domesticate: false, // Would need special mechanics
        living_products: vec![],
    }
}

// ============================================================================
// MEDIUM HERBIVORES
// ============================================================================

fn deer() -> AnimalSpecies {
    AnimalSpecies {
        id: "deer".to_string(),
        name: "Deer".to_string(),
        description: "Graceful herbivore, provides quality leather and meat".to_string(),
        health: 60.0,
        attack_damage: 5.0,
        defense: 1.0,
        speed: 1.8,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (2, 6),
        drops: vec![
            AnimalDrop::new("deer_meat".to_string(), 8, 12),
            AnimalDrop::new("leather".to_string(), 4, 6),
            AnimalDrop::new("antler".to_string(), 2, 2).with_chance(0.5), // Only males
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn sheep() -> AnimalSpecies {
    AnimalSpecies {
        id: "sheep".to_string(),
        name: "Sheep".to_string(),
        description: "Docile wool-producing livestock".to_string(),
        health: 50.0,
        attack_damage: 2.0,
        defense: 1.0,
        speed: 1.0,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        group_size: (4, 12),
        drops: vec![
            AnimalDrop::new("mutton".to_string(), 6, 10),
            AnimalDrop::new("leather".to_string(), 3, 5),
            AnimalDrop::new("wool".to_string(), 4, 6),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "wool".to_string(),
                production_time: 600, // Shear every 600 ticks
                quantity: 4,
            },
        ],
    }
}

fn goat() -> AnimalSpecies {
    AnimalSpecies {
        id: "goat".to_string(),
        name: "Goat".to_string(),
        description: "Hardy mountain animal, produces milk and leather".to_string(),
        health: 55.0,
        attack_damage: 6.0,
        defense: 2.0,
        speed: 1.3,
        behavior: AnimalBehavior::Defensive,
        diet: DietType::Herbivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        group_size: (3, 8),
        drops: vec![
            AnimalDrop::new("goat_meat".to_string(), 5, 8),
            AnimalDrop::new("leather".to_string(), 4, 6),
            AnimalDrop::new("horn".to_string(), 2, 2).with_chance(0.7),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "milk".to_string(),
                production_time: 200,
                quantity: 1,
            },
        ],
    }
}

// ============================================================================
// MEDIUM/LARGE OMNIVORES & LIVESTOCK
// ============================================================================

fn boar() -> AnimalSpecies {
    AnimalSpecies {
        id: "boar".to_string(),
        name: "Wild Boar".to_string(),
        description: "Aggressive omnivore with thick hide, dangerous when provoked".to_string(),
        health: 80.0,
        attack_damage: 15.0,
        defense: 5.0,
        speed: 1.4,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Omnivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (1, 4),
        drops: vec![
            AnimalDrop::new("pork".to_string(), 10, 15),
            AnimalDrop::new("thick_hide".to_string(), 4, 6),
            AnimalDrop::new("leather".to_string(), 3, 5),
            AnimalDrop::new("boar_tusk".to_string(), 2, 2).with_chance(0.6),
        ],
        can_domesticate: true, // Becomes pig
        living_products: vec![],
    }
}

fn cow() -> AnimalSpecies {
    AnimalSpecies {
        id: "cow".to_string(),
        name: "Cow".to_string(),
        description: "Large domesticated livestock, provides milk, meat, and leather".to_string(),
        health: 100.0,
        attack_damage: 8.0,
        defense: 3.0,
        speed: 0.9,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (2, 8),
        drops: vec![
            AnimalDrop::new("beef".to_string(), 15, 25),
            AnimalDrop::new("leather".to_string(), 8, 12),
            AnimalDrop::new("thick_hide".to_string(), 2, 4),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "milk".to_string(),
                production_time: 150,
                quantity: 2,
            },
        ],
    }
}

// ============================================================================
// LARGE PREDATORS
// ============================================================================

fn bear() -> AnimalSpecies {
    AnimalSpecies {
        id: "bear".to_string(),
        name: "Bear".to_string(),
        description: "Massive predator, extremely dangerous, provides thick fur and hide".to_string(),
        health: 200.0,
        attack_damage: 30.0,
        defense: 8.0,
        speed: 1.3,
        behavior: AnimalBehavior::Territorial,
        diet: DietType::Omnivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("bear_meat".to_string(), 20, 30),
            AnimalDrop::new("fur".to_string(), 10, 15),
            AnimalDrop::new("thick_hide".to_string(), 8, 12),
            AnimalDrop::new("leather".to_string(), 6, 10),
            AnimalDrop::new("bear_claw".to_string(), 4, 4),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn lion() -> AnimalSpecies {
    AnimalSpecies {
        id: "lion".to_string(),
        name: "Lion".to_string(),
        description: "Apex predator of hot climates, hunts in prides".to_string(),
        health: 180.0,
        attack_damage: 28.0,
        defense: 6.0,
        speed: 1.9,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Desert],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (3, 8),
        drops: vec![
            AnimalDrop::new("lion_meat".to_string(), 18, 25),
            AnimalDrop::new("fur".to_string(), 8, 12),
            AnimalDrop::new("thick_hide".to_string(), 6, 10),
            AnimalDrop::new("lion_fang".to_string(), 2, 4),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

// ============================================================================
// BIOME SPECIALISTS
// ============================================================================

fn arctic_fox() -> AnimalSpecies {
    AnimalSpecies {
        id: "arctic_fox".to_string(),
        name: "Arctic Fox".to_string(),
        description: "White-furred fox adapted to extreme cold".to_string(),
        health: 35.0,
        attack_damage: 7.0,
        defense: 2.0,
        speed: 1.7,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (1, 2),
        drops: vec![
            AnimalDrop::new("fox_meat".to_string(), 2, 3),
            AnimalDrop::new("fur".to_string(), 3, 5), // More fur than regular fox
            AnimalDrop::new("leather".to_string(), 1, 2),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn camel() -> AnimalSpecies {
    AnimalSpecies {
        id: "camel".to_string(),
        name: "Camel".to_string(),
        description: "Desert beast of burden, stores water and provides transport".to_string(),
        health: 120.0,
        attack_damage: 10.0,
        defense: 4.0,
        speed: 1.1,
        behavior: AnimalBehavior::Defensive,
        diet: DietType::Herbivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Desert],
        secondary_biomes: vec![],
        group_size: (2, 6),
        drops: vec![
            AnimalDrop::new("camel_meat".to_string(), 15, 20),
            AnimalDrop::new("leather".to_string(), 10, 15),
            AnimalDrop::new("thick_hide".to_string(), 4, 6),
            AnimalDrop::new("fur".to_string(), 4, 6), // Camel hair
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "milk".to_string(),
                production_time: 300,
                quantity: 1,
            },
        ],
    }
}

fn mammoth() -> AnimalSpecies {
    AnimalSpecies {
        id: "mammoth".to_string(),
        name: "Woolly Mammoth".to_string(),
        description: "Massive ice age giant with long tusks and thick fur".to_string(),
        health: 300.0,
        attack_damage: 40.0,
        defense: 10.0,
        speed: 0.8,
        behavior: AnimalBehavior::Territorial,
        diet: DietType::Herbivore,
        size: AnimalSize::Huge,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (2, 8),
        drops: vec![
            AnimalDrop::new("mammoth_meat".to_string(), 40, 60),
            AnimalDrop::new("fur".to_string(), 20, 30),
            AnimalDrop::new("thick_hide".to_string(), 15, 25),
            AnimalDrop::new("leather".to_string(), 10, 15),
            AnimalDrop::new("ivory_tusk".to_string(), 2, 2),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

// ============================================================================
// ADDITIONAL DOMESTIC ANIMALS
// ============================================================================

fn duck() -> AnimalSpecies {
    AnimalSpecies {
        id: "duck".to_string(),
        name: "Duck".to_string(),
        description: "Waterfowl, provides eggs, meat, and feathers".to_string(),
        health: 10.0,
        attack_damage: 0.5,
        defense: 0.0,
        speed: 1.3,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Omnivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (4, 10),
        drops: vec![
            AnimalDrop::new("duck_meat".to_string(), 2, 3),
            AnimalDrop::new("feathers".to_string(), 4, 6),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "egg".to_string(),
                production_time: 120,
                quantity: 1,
            },
        ],
    }
}

fn goose() -> AnimalSpecies {
    AnimalSpecies {
        id: "goose".to_string(),
        name: "Goose".to_string(),
        description: "Large waterfowl, aggressive when defending territory".to_string(),
        health: 15.0,
        attack_damage: 3.0,
        defense: 1.0,
        speed: 1.2,
        behavior: AnimalBehavior::Defensive,
        diet: DietType::Herbivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        group_size: (5, 12),
        drops: vec![
            AnimalDrop::new("goose_meat".to_string(), 3, 4),
            AnimalDrop::new("feathers".to_string(), 6, 10), // More feathers than duck
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "egg".to_string(),
                production_time: 150,
                quantity: 1,
            },
        ],
    }
}

fn pig() -> AnimalSpecies {
    AnimalSpecies {
        id: "pig".to_string(),
        name: "Pig".to_string(),
        description: "Domesticated boar, excellent meat source".to_string(),
        health: 60.0,
        attack_damage: 5.0,
        defense: 2.0,
        speed: 1.1,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Omnivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (3, 8),
        drops: vec![
            AnimalDrop::new("pork".to_string(), 12, 18),
            AnimalDrop::new("leather".to_string(), 4, 6),
        ],
        can_domesticate: true,
        living_products: vec![],
    }
}

// ============================================================================
// BIRDS
// ============================================================================

fn crow() -> AnimalSpecies {
    AnimalSpecies {
        id: "crow".to_string(),
        name: "Crow".to_string(),
        description: "Intelligent scavenger bird, often found near settlements".to_string(),
        health: 8.0,
        attack_damage: 2.0,
        defense: 0.0,
        speed: 2.0, // Very fast (flying)
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Omnivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert, ClimateZone::Arctic],
        group_size: (3, 15),
        drops: vec![
            AnimalDrop::new("crow_meat".to_string(), 1, 1),
            AnimalDrop::new("feathers".to_string(), 2, 3),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn eagle() -> AnimalSpecies {
    AnimalSpecies {
        id: "eagle".to_string(),
        name: "Eagle".to_string(),
        description: "Majestic bird of prey, hunts from great heights".to_string(),
        health: 25.0,
        attack_damage: 10.0,
        defense: 1.0,
        speed: 2.5, // Extremely fast (flying)
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert, ClimateZone::Arctic],
        group_size: (1, 2),
        drops: vec![
            AnimalDrop::new("bird_meat".to_string(), 2, 3),
            AnimalDrop::new("feathers".to_string(), 4, 6),
            AnimalDrop::new("eagle_talon".to_string(), 2, 2).with_chance(0.8),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn hawk() -> AnimalSpecies {
    AnimalSpecies {
        id: "hawk".to_string(),
        name: "Hawk".to_string(),
        description: "Swift predatory bird, can be trained for hunting".to_string(),
        health: 20.0,
        attack_damage: 8.0,
        defense: 1.0,
        speed: 2.3,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Desert],
        secondary_biomes: vec![],
        group_size: (1, 2),
        drops: vec![
            AnimalDrop::new("bird_meat".to_string(), 2, 3),
            AnimalDrop::new("feathers".to_string(), 3, 5),
        ],
        can_domesticate: true, // Falconry
        living_products: vec![],
    }
}

fn owl() -> AnimalSpecies {
    AnimalSpecies {
        id: "owl".to_string(),
        name: "Owl".to_string(),
        description: "Nocturnal hunter, silent and deadly".to_string(),
        health: 18.0,
        attack_damage: 7.0,
        defense: 1.0,
        speed: 2.0,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("bird_meat".to_string(), 1, 2),
            AnimalDrop::new("feathers".to_string(), 3, 5),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn parrot() -> AnimalSpecies {
    AnimalSpecies {
        id: "parrot".to_string(),
        name: "Parrot".to_string(),
        description: "Colorful tropical bird, intelligent and vocal".to_string(),
        health: 12.0,
        attack_damage: 3.0,
        defense: 0.0,
        speed: 1.8,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Omnivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        group_size: (2, 8),
        drops: vec![
            AnimalDrop::new("bird_meat".to_string(), 1, 2),
            AnimalDrop::new("feathers".to_string(), 4, 6),
        ],
        can_domesticate: true, // As pets
        living_products: vec![],
    }
}

// ============================================================================
// MORE PREDATORS
// ============================================================================

fn snake() -> AnimalSpecies {
    AnimalSpecies {
        id: "snake".to_string(),
        name: "Snake".to_string(),
        description: "Venomous reptile, dangerous despite small size".to_string(),
        health: 20.0,
        attack_damage: 15.0, // Venomous
        defense: 1.0,
        speed: 1.2,
        behavior: AnimalBehavior::Defensive,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Desert, ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("snake_meat".to_string(), 2, 3),
            AnimalDrop::new("snake_skin".to_string(), 1, 2),
            AnimalDrop::new("venom_sac".to_string(), 1, 1).with_chance(0.5),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn tiger() -> AnimalSpecies {
    AnimalSpecies {
        id: "tiger".to_string(),
        name: "Tiger".to_string(),
        description: "Apex predator of jungles, solitary and deadly".to_string(),
        health: 190.0,
        attack_damage: 32.0,
        defense: 7.0,
        speed: 2.0,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("tiger_meat".to_string(), 18, 25),
            AnimalDrop::new("fur".to_string(), 10, 15),
            AnimalDrop::new("thick_hide".to_string(), 6, 10),
            AnimalDrop::new("tiger_fang".to_string(), 2, 4),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn crocodile() -> AnimalSpecies {
    AnimalSpecies {
        id: "crocodile".to_string(),
        name: "Crocodile".to_string(),
        description: "Ancient reptilian predator, lurks in water".to_string(),
        health: 150.0,
        attack_damage: 35.0,
        defense: 12.0, // Armored scales
        speed: 0.9, // Slow on land
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Desert],
        group_size: (1, 3),
        drops: vec![
            AnimalDrop::new("crocodile_meat".to_string(), 15, 20),
            AnimalDrop::new("crocodile_scales".to_string(), 10, 15),
            AnimalDrop::new("thick_hide".to_string(), 8, 12),
            AnimalDrop::new("crocodile_tooth".to_string(), 4, 8),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn polar_bear() -> AnimalSpecies {
    AnimalSpecies {
        id: "polar_bear".to_string(),
        name: "Polar Bear".to_string(),
        description: "Massive arctic predator, adapted to extreme cold".to_string(),
        health: 220.0,
        attack_damage: 35.0,
        defense: 9.0,
        speed: 1.4,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("bear_meat".to_string(), 22, 35),
            AnimalDrop::new("fur".to_string(), 15, 20), // Extra warm fur
            AnimalDrop::new("thick_hide".to_string(), 10, 15),
            AnimalDrop::new("bear_claw".to_string(), 4, 4),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

// ============================================================================
// LARGE HERBIVORES (MOUNT-COMPATIBLE)
// ============================================================================

fn elk_animal() -> AnimalSpecies {
    AnimalSpecies {
        id: "elk".to_string(),
        name: "Elk".to_string(),
        description: "Large forest herbivore with impressive antlers".to_string(),
        health: 90.0,
        attack_damage: 12.0,
        defense: 3.0,
        speed: 1.6,
        behavior: AnimalBehavior::Defensive,
        diet: DietType::Herbivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (3, 10),
        drops: vec![
            AnimalDrop::new("elk_meat".to_string(), 12, 18),
            AnimalDrop::new("leather".to_string(), 6, 10),
            AnimalDrop::new("antler".to_string(), 2, 2).with_chance(0.6),
        ],
        can_domesticate: true, // Can be trained as mount
        living_products: vec![],
    }
}

fn reindeer_animal() -> AnimalSpecies {
    AnimalSpecies {
        id: "reindeer".to_string(),
        name: "Reindeer".to_string(),
        description: "Arctic herbivore, adapted to snow and cold".to_string(),
        health: 70.0,
        attack_damage: 8.0,
        defense: 2.0,
        speed: 1.7,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (5, 20),
        drops: vec![
            AnimalDrop::new("reindeer_meat".to_string(), 10, 15),
            AnimalDrop::new("leather".to_string(), 5, 8),
            AnimalDrop::new("fur".to_string(), 4, 6),
            AnimalDrop::new("antler".to_string(), 2, 2).with_chance(0.7),
        ],
        can_domesticate: true, // Can be trained as mount
        living_products: vec![],
    }
}

// ============================================================================
// TROPICAL SPECIALISTS
// ============================================================================

fn monkey() -> AnimalSpecies {
    AnimalSpecies {
        id: "monkey".to_string(),
        name: "Monkey".to_string(),
        description: "Agile tree-dweller, intelligent and mischievous".to_string(),
        health: 25.0,
        attack_damage: 5.0,
        defense: 1.0,
        speed: 1.9,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Omnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        group_size: (5, 15),
        drops: vec![
            AnimalDrop::new("monkey_meat".to_string(), 2, 4),
            AnimalDrop::new("fur".to_string(), 1, 2),
        ],
        can_domesticate: true, // As companions/pets
        living_products: vec![],
    }
}

// ============================================================================
// AQUATIC ANIMALS
// ============================================================================

fn fish() -> AnimalSpecies {
    AnimalSpecies {
        id: "fish".to_string(),
        name: "Fish".to_string(),
        description: "Common fish, found in rivers and lakes".to_string(),
        health: 5.0,
        attack_damage: 0.0,
        defense: 0.0,
        speed: 1.5,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Carnivore, // Eat smaller fish
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (10, 50),
        drops: vec![
            AnimalDrop::new("fish_meat".to_string(), 1, 2),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn otter() -> AnimalSpecies {
    AnimalSpecies {
        id: "otter".to_string(),
        name: "Otter".to_string(),
        description: "Playful aquatic mammal, hunts fish".to_string(),
        health: 30.0,
        attack_damage: 6.0,
        defense: 1.0,
        speed: 1.6,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (2, 6),
        drops: vec![
            AnimalDrop::new("otter_meat".to_string(), 3, 5),
            AnimalDrop::new("fur".to_string(), 3, 5), // Water-resistant fur
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

fn seal() -> AnimalSpecies {
    AnimalSpecies {
        id: "seal".to_string(),
        name: "Seal".to_string(),
        description: "Arctic aquatic mammal, thick blubber provides warmth".to_string(),
        health: 80.0,
        attack_damage: 8.0,
        defense: 4.0,
        speed: 1.1, // Slow on land, fast in water
        behavior: AnimalBehavior::Passive,
        diet: DietType::Carnivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (3, 12),
        drops: vec![
            AnimalDrop::new("seal_meat".to_string(), 10, 15),
            AnimalDrop::new("blubber".to_string(), 8, 12), // Fat/oil
            AnimalDrop::new("fur".to_string(), 4, 6),
            AnimalDrop::new("leather".to_string(), 3, 5),
        ],
        can_domesticate: false,
        living_products: vec![],
    }
}

// ============================================================================
// ANIMAL INSTANCE SYSTEM (Individual animals in the world)
// ============================================================================

/// AI state for animal behavior
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimalState {
    /// Wandering aimlessly
    Idle,
    /// Moving towards food/grazing
    Grazing,
    /// Seeking water
    Drinking,
    /// Resting to recover health/stamina
    Resting,
    /// Following herd/pack
    Following,
    /// Hunting prey
    Hunting { target_id: Option<Uuid> },
    /// Fleeing from danger
    Fleeing { from_position: (i32, i32) },
    /// Attacking threat
    Attacking { target_id: Uuid },
    /// Dead
    Dead,
}

/// Individual animal instance in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Animal {
    pub id: Uuid,
    pub species_id: String,

    /// Position in world
    pub position: (i32, i32),
    pub facing: f32, // Direction in radians

    /// Stats
    pub current_health: f32,
    pub max_health: f32,
    pub stamina: f32,
    pub max_stamina: f32,

    /// AI state
    pub state: AnimalState,
    pub state_timer: u32, // Ticks remaining in current state

    /// Herd/pack affiliation
    pub group_id: Option<Uuid>,

    /// Age (in ticks)
    pub age: u32,
    pub maturity_age: u32, // Age when fully grown

    /// Domestication
    pub is_domesticated: bool,
    pub tame_level: f32, // 0.0 = wild, 1.0 = fully tamed
    pub owner_id: Option<Uuid>, // Agent who owns this animal

    /// Reproduction
    pub can_reproduce: bool,
    pub reproduction_cooldown: u32,

    /// Living product timers
    pub product_timers: HashMap<String, u32>, // material_id -> ticks until production
}

impl Animal {
    pub fn new(species_id: String, position: (i32, i32), species: &AnimalSpecies) -> Self {
        let mut product_timers = HashMap::new();
        for product in &species.living_products {
            product_timers.insert(product.material_id.clone(), product.production_time);
        }

        Self {
            id: Uuid::new_v4(),
            species_id,
            position,
            facing: 0.0,
            current_health: species.health,
            max_health: species.health,
            stamina: 100.0,
            max_stamina: 100.0,
            state: AnimalState::Idle,
            state_timer: 0,
            group_id: None,
            age: 0,
            maturity_age: 1000, // Default 1000 ticks to mature
            is_domesticated: false,
            tame_level: 0.0,
            owner_id: None,
            can_reproduce: true,
            reproduction_cooldown: 0,
            product_timers,
        }
    }

    /// Check if animal is alive
    pub fn is_alive(&self) -> bool {
        self.current_health > 0.0 && self.state != AnimalState::Dead
    }

    /// Check if animal is mature (can reproduce, full stats)
    pub fn is_mature(&self) -> bool {
        self.age >= self.maturity_age
    }

    /// Check if animal is wild (not domesticated)
    pub fn is_wild(&self) -> bool {
        !self.is_domesticated
    }

    /// Damage the animal
    pub fn take_damage(&mut self, amount: f32) {
        self.current_health = (self.current_health - amount).max(0.0);
        if self.current_health == 0.0 {
            self.state = AnimalState::Dead;
        }
    }

    /// Heal the animal
    pub fn heal(&mut self, amount: f32) {
        if self.is_alive() {
            self.current_health = (self.current_health + amount).min(self.max_health);
        }
    }

    /// Consume stamina
    pub fn use_stamina(&mut self, amount: f32) {
        self.stamina = (self.stamina - amount).max(0.0);
    }

    /// Recover stamina
    pub fn recover_stamina(&mut self, amount: f32) {
        self.stamina = (self.stamina + amount).min(self.max_stamina);
    }

    /// Check if exhausted
    pub fn is_exhausted(&self) -> bool {
        self.stamina < 20.0
    }

    /// Tame the animal (increase tame level)
    pub fn tame(&mut self, amount: f32) {
        if !self.is_domesticated {
            self.tame_level = (self.tame_level + amount).min(1.0);
            if self.tame_level >= 1.0 {
                self.is_domesticated = true;
            }
        }
    }

    /// Tick product production timers and return ready products
    pub fn tick_products(&mut self) -> Vec<(String, u32)> {
        let mut produced = Vec::new();

        if !self.is_alive() || !self.is_mature() {
            return produced;
        }

        for (material_id, timer) in self.product_timers.iter_mut() {
            if *timer > 0 {
                *timer -= 1;
            } else {
                // Find the product info to get quantity
                // We'll return the material_id and quantity
                produced.push((material_id.clone(), 1)); // Default quantity
                *timer = 100; // Reset timer (will be updated with actual value)
            }
        }

        produced
    }

    /// Age the animal by one tick
    pub fn tick_age(&mut self) {
        self.age += 1;

        // Update reproduction cooldown
        if self.reproduction_cooldown > 0 {
            self.reproduction_cooldown -= 1;
        }
    }

    /// Get health percentage
    pub fn health_percentage(&self) -> f32 {
        self.current_health / self.max_health
    }

    /// Get stamina percentage
    pub fn stamina_percentage(&self) -> f32 {
        self.stamina / self.max_stamina
    }
}

// ============================================================================
// ANIMAL MANAGER (Manages all animals in the world)
// ============================================================================

/// Manages animal population and AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimalManager {
    animals: Vec<Animal>,
    groups: HashMap<Uuid, Vec<Uuid>>, // Group ID -> Animal IDs

    /// Spawning parameters
    spawn_rate: f32, // Chance per tick to spawn
    max_population: usize,

    /// Reference to fauna registry (not serialized)
    #[serde(skip)]
    registry: Option<FaunaRegistry>,
}

impl AnimalManager {
    pub fn new(max_population: usize) -> Self {
        Self {
            animals: Vec::new(),
            groups: HashMap::new(),
            spawn_rate: 0.001, // 0.1% chance per tick
            max_population,
            registry: Some(FaunaRegistry::new()),
        }
    }

    /// Spawn an animal at a position
    pub fn spawn_animal(&mut self, species_id: String, position: (i32, i32)) -> Option<Uuid> {
        if self.animals.len() >= self.max_population {
            return None;
        }

        let species = self.registry.as_ref()?.get(&species_id)?;
        let animal = Animal::new(species_id, position, species);
        let id = animal.id;
        self.animals.push(animal);
        Some(id)
    }

    /// Spawn a herd/pack of animals
    pub fn spawn_group(&mut self, species_id: String, center: (i32, i32), count: u32) -> Option<Uuid> {
        let group_id = Uuid::new_v4();
        let mut members = Vec::new();

        let species = self.registry.as_ref()?.get(&species_id)?;

        for i in 0..count {
            if self.animals.len() >= self.max_population {
                break;
            }

            // Spawn in a circle around center
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let radius = 3.0;
            let x = center.0 + (angle.cos() * radius) as i32;
            let y = center.1 + (angle.sin() * radius) as i32;

            let mut animal = Animal::new(species_id.clone(), (x, y), species);
            animal.group_id = Some(group_id);

            members.push(animal.id);
            self.animals.push(animal);
        }

        if !members.is_empty() {
            self.groups.insert(group_id, members);
            Some(group_id)
        } else {
            None
        }
    }

    /// Get all animals
    pub fn get_all(&self) -> &Vec<Animal> {
        &self.animals
    }

    /// Get specific animal
    pub fn get(&self, id: &Uuid) -> Option<&Animal> {
        self.animals.iter().find(|a| a.id == *id)
    }

    /// Get mutable animal
    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut Animal> {
        self.animals.iter_mut().find(|a| a.id == *id)
    }

    /// Get all animals at a position
    pub fn get_at_position(&self, position: (i32, i32)) -> Vec<&Animal> {
        self.animals.iter()
            .filter(|a| a.position == position && a.is_alive())
            .collect()
    }

    /// Get animals in radius
    pub fn get_in_radius(&self, center: (i32, i32), radius: f32) -> Vec<&Animal> {
        self.animals.iter()
            .filter(|a| {
                if !a.is_alive() {
                    return false;
                }
                let dx = (a.position.0 - center.0) as f32;
                let dy = (a.position.1 - center.1) as f32;
                (dx * dx + dy * dy).sqrt() <= radius
            })
            .collect()
    }

    /// Get animals of a specific species
    pub fn get_by_species(&self, species_id: &str) -> Vec<&Animal> {
        self.animals.iter()
            .filter(|a| a.species_id == species_id && a.is_alive())
            .collect()
    }

    /// Get domesticated animals owned by an agent
    pub fn get_owned_by(&self, owner_id: &Uuid) -> Vec<&Animal> {
        self.animals.iter()
            .filter(|a| a.owner_id == Some(*owner_id) && a.is_alive())
            .collect()
    }

    /// Get species from registry
    pub fn get_species(&self, species_id: &str) -> Option<&AnimalSpecies> {
        self.registry.as_ref()?.get(species_id)
    }

    /// Remove dead animals
    pub fn remove_dead(&mut self) {
        self.animals.retain(|a| a.is_alive());
    }

    /// Tick all animals (age, products, natural healing, AI behaviors)
    pub fn tick(&mut self) {
        let registry = match &self.registry {
            Some(r) => r,
            None => return,
        };

        // First pass: basic updates
        for animal in &mut self.animals {
            if !animal.is_alive() {
                continue;
            }

            // Age
            animal.tick_age();

            // Natural stamina recovery when resting
            if animal.state == AnimalState::Resting {
                animal.recover_stamina(1.0);
            } else if animal.state != AnimalState::Dead {
                // Gradual stamina consumption for active animals
                animal.use_stamina(0.1);
            }

            // Slow natural healing
            if animal.current_health < animal.max_health {
                animal.heal(0.1);
            }

            // Tick products
            animal.tick_products();

            // Decrement state timer
            if animal.state_timer > 0 {
                animal.state_timer -= 1;
            }
        }

        // Second pass: AI behavior (needs mutable access and species lookup)
        let animals_data: Vec<(usize, String, AnimalBehavior, bool)> = self.animals
            .iter()
            .enumerate()
            .filter(|(_, a)| a.is_alive())
            .filter_map(|(idx, a)| {
                let species = registry.get(&a.species_id)?;
                Some((idx, a.species_id.clone(), species.behavior, a.is_wild()))
            })
            .collect();

        for (idx, _species_id, behavior, is_wild) in animals_data {
            self.update_animal_behavior(idx, behavior, is_wild);
        }
    }

    /// Update individual animal AI behavior
    fn update_animal_behavior(&mut self, animal_idx: usize, behavior: AnimalBehavior, is_wild: bool) {
        let animal = &mut self.animals[animal_idx];

        // If state timer is active, continue current behavior
        if animal.state_timer > 0 {
            return;
        }

        // Transition to new state based on behavior type and conditions
        match behavior {
            AnimalBehavior::Passive => {
                // Passive animals: graze, rest, or idle
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 50;
                } else if rand::random::<f32>() < 0.3 {
                    animal.state = AnimalState::Grazing;
                    animal.state_timer = 30;
                    // Move slightly while grazing
                    let offset = (rand::random::<i32>() % 3 - 1, rand::random::<i32>() % 3 - 1);
                    animal.position.0 += offset.0;
                    animal.position.1 += offset.1;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 20;
                }
            }
            AnimalBehavior::Neutral => {
                // Neutral animals: idle, drink, or graze
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 40;
                } else if rand::random::<f32>() < 0.2 {
                    animal.state = AnimalState::Drinking;
                    animal.state_timer = 25;
                } else if rand::random::<f32>() < 0.4 {
                    animal.state = AnimalState::Grazing;
                    animal.state_timer = 30;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 25;
                }
            }
            AnimalBehavior::Defensive => {
                // Defensive animals: mostly graze but ready to react
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 45;
                } else if rand::random::<f32>() < 0.5 {
                    animal.state = AnimalState::Grazing;
                    animal.state_timer = 35;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 20;
                }
            }
            AnimalBehavior::Aggressive | AnimalBehavior::Territorial => {
                // Aggressive/territorial animals: hunt or patrol
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 60;
                } else if is_wild && rand::random::<f32>() < 0.3 {
                    // Wild predators occasionally hunt
                    animal.state = AnimalState::Hunting { target_id: None };
                    animal.state_timer = 50;
                    // Patrol/move while hunting
                    let offset = (rand::random::<i32>() % 5 - 2, rand::random::<i32>() % 5 - 2);
                    animal.position.0 += offset.0;
                    animal.position.1 += offset.1;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 30;
                }
            }
        }
    }

    /// Count living animals
    pub fn population_count(&self) -> usize {
        self.animals.iter().filter(|a| a.is_alive()).count()
    }

    /// Count animals by species
    pub fn count_by_species(&self, species_id: &str) -> usize {
        self.animals.iter()
            .filter(|a| a.species_id == species_id && a.is_alive())
            .count()
    }

    /// Get total living animals by behavior
    pub fn count_by_behavior(&self, behavior: AnimalBehavior) -> usize {
        let registry = match &self.registry {
            Some(r) => r,
            None => return 0,
        };

        self.animals.iter()
            .filter(|a| {
                if !a.is_alive() {
                    return false;
                }
                registry.get(&a.species_id)
                    .map(|s| s.behavior == behavior)
                    .unwrap_or(false)
            })
            .count()
    }
}

impl Default for AnimalManager {
    fn default() -> Self {
        Self::new(1000) // Default max 1000 animals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fauna_registry() {
        let registry = FaunaRegistry::new();

        assert!(registry.get("rabbit").is_some());
        assert!(registry.get("bear").is_some());
        assert!(registry.get("sheep").is_some());
    }

    #[test]
    fn test_biome_filtering() {
        let registry = FaunaRegistry::new();

        let arctic_animals = registry.get_by_biome(ClimateZone::Arctic);
        assert!(!arctic_animals.is_empty());

        // Mammoth should be in arctic
        assert!(arctic_animals.iter().any(|a| a.id == "mammoth"));
    }

    #[test]
    fn test_behavior_filtering() {
        let registry = FaunaRegistry::new();

        let aggressive = registry.get_by_behavior(AnimalBehavior::Aggressive);
        assert!(!aggressive.is_empty());

        // Wolf and bear should be aggressive/territorial
        assert!(aggressive.iter().any(|a| a.id == "wolf"));
    }

    #[test]
    fn test_domestication() {
        let sheep = sheep();
        let bear = bear();

        assert!(sheep.can_domesticate);
        assert!(!bear.can_domesticate);
    }

    #[test]
    fn test_living_products() {
        let sheep = sheep();
        let rabbit = rabbit();

        assert!(!sheep.living_products.is_empty());
        assert!(rabbit.living_products.is_empty());

        // Sheep should produce wool
        assert!(sheep.living_products.iter().any(|p| p.material_id == "wool"));
    }

    #[test]
    fn test_animal_drops() {
        let bear = bear();

        // Bear should drop fur and hide
        assert!(bear.drops.iter().any(|d| d.material_id == "fur"));
        assert!(bear.drops.iter().any(|d| d.material_id == "thick_hide"));
    }

    #[test]
    fn test_size_categories() {
        let rabbit = rabbit();
        let deer = deer();
        let mammoth = mammoth();

        assert_eq!(rabbit.size, AnimalSize::Tiny);
        assert_eq!(deer.size, AnimalSize::Medium);
        assert_eq!(mammoth.size, AnimalSize::Huge);
    }
}
