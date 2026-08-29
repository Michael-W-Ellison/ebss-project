// src/environment/fauna.rs
//! Animal life and wildlife system with biome distributions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::flora::ClimateZone;
use crate::world::{Grid, TerrainType};
use uuid::Uuid;

/// Maps terrain type to the most likely climate zone for that terrain
pub fn terrain_to_climate_zone(terrain: TerrainType) -> ClimateZone {
    match terrain {
        TerrainType::Desert => ClimateZone::Desert,
        // Most terrain types are temperate
        TerrainType::Plains
        | TerrainType::Forest
        | TerrainType::Hills
        | TerrainType::Meadow
        | TerrainType::Wetland
        | TerrainType::Riverbank
        | TerrainType::Beach
        | TerrainType::Farmland
        | TerrainType::Water
        | TerrainType::Sea
        | TerrainType::SaltMarsh => ClimateZone::Temperate,
        // A salt flat is a shallow sea that dried up, and it dried up for a
        // reason
        TerrainType::SaltFlat => ClimateZone::Desert,
        // Mountains can be cold (arctic adjacent)
        TerrainType::Mountain => ClimateZone::Arctic,
    }
}

/// How much longer an animal waits between litters than its species data says.
///
/// The species numbers give a sheep about eight litters in a lifetime and a
/// wolf about seven. At that rate a herd of forty needs some thirty wolves to
/// hold it level - an inverted pyramid, and one the spawn ratio of four prey
/// groups to one predator group can never supply. Stretching the interval
/// brings herd growth back within what a plausible number of predators can
/// take.
const BREEDING_INTERVAL_SCALE: f32 = 3.0;

/// Side of the patches the world is divided into when asking how crowded a
/// piece of ground is
const GRAZING_PATCH: i32 = 6;

/// How many animals a patch of that size will carry before the ones on it stop
/// breeding
const PATCH_CARRYING_CAPACITY: u32 = 8;

/// Configuration for naturalistic animal spawning during world generation
#[derive(Debug, Clone)]
pub struct AnimalSpawnConfig {
    /// Base number of herds/groups to spawn per 100x100 tiles
    pub herds_per_10000_tiles: usize,
    /// Whether to spawn predators
    pub spawn_predators: bool,
    /// Ratio of prey to predator groups
    pub prey_to_predator_ratio: f32,
    /// Maximum initial population cap
    pub max_initial_population: usize,
}

impl Default for AnimalSpawnConfig {
    fn default() -> Self {
        Self {
            // Eight herds per ten thousand tiles is two herds on the default
            // fifty-by-fifty world: one of prey and one of predators, four
            // sheep and a fox. Nothing balances at that size - the predators
            // die out and the herbivores run to the population cap - so the
            // density is set to give a world that can actually hold a
            // predator and a prey population at once.
            herds_per_10000_tiles: 40,
            spawn_predators: true,
            // Two prey groups per predator group rather than four. A world
            // that starts with one or two predators loses them to bad luck -
            // a random walk apart, one lean winter - and once they are gone
            // nothing holds the herds at all. What the herds breed needs a
            // pack that can actually keep up with them.
            prey_to_predator_ratio: 2.0,
            max_initial_population: 200,
        }
    }
}

/// Animal behavior classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimalBehavior {
    Passive,    // Flees from threats
    Neutral,    // Ignores unless provoked
    Defensive,  // Attacks when cornered
    Aggressive, // Attacks on sight
    Territorial, // Attacks near den/territory
}

impl AnimalBehavior {
    /// How much of a thing's teeth count against somebody who is simply
    /// standing there.
    ///
    /// A rabbit has an `attack_damage` of 1.0 and a deer of 5.0, because both
    /// will defend themselves if you go at them. Neither is a threat to a man
    /// walking past, and reading danger off `attack_damage` alone said they
    /// were: once several of a thing began adding up, a herd of twenty
    /// reindeer registered as about as dangerous as a wolf.
    ///
    /// What menaces somebody who has done nothing is a thing that comes after
    /// people. What defends itself is a question for whoever attacks it.
    ///
    /// What it cost, measured over twenty-four worlds: a settlement ran 465
    /// times where it should have run 213, and froze 194 times where it should
    /// have frozen 27 - most of that being children hemmed in by deer.
    /// How readily a thing of this temper turns and faces what is coming at
    /// it, rather than running.
    ///
    /// The other side of `how_much_it_menaces_you`, and the whole of an
    /// animal's courage. A rabbit never stands its ground whatever the odds
    /// are - that is what Passive means, and a rabbit that fights a wolf
    /// because the arithmetic came out that way is not a rabbit. Everything
    /// else weighs the odds, and weighs them more kindly the fiercer it is.
    pub fn how_readily_it_stands_its_ground(&self) -> f32 {
        match self {
            AnimalBehavior::Passive => 0.0,
            AnimalBehavior::Neutral => 0.6,
            AnimalBehavior::Defensive => 0.9,
            AnimalBehavior::Aggressive => 1.2,
            AnimalBehavior::Territorial => 1.3,
        }
    }

    pub fn how_much_it_menaces_you(&self) -> f32 {
        match self {
            // Runs away. Not a threat to anybody, whatever it would do if
            // cornered
            AnimalBehavior::Passive => 0.0,
            // Minds its own business, and is worth an eye
            AnimalBehavior::Neutral => 0.25,
            // Will not start it, but is a bad thing to be near
            AnimalBehavior::Defensive => 0.4,
            // Comes after you
            AnimalBehavior::Aggressive | AnimalBehavior::Territorial => 1.0,
        }
    }
}

/// Animal diet type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DietType {
    Herbivore,
    Carnivore,
    Omnivore,
}

/// Size classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

    // === LIFECYCLE FIELDS ===
    /// Lifespan in ticks (min, max) - animals die of old age
    pub lifespan: (u32, u32),
    /// Age at which animal reaches maturity
    pub maturity_age: u32,
    /// Breeding cooldown in ticks after reproduction
    pub breeding_cooldown: u32,
    /// Gestation period in ticks (0 for egg-layers)
    pub gestation_period: u32,
    /// Number of offspring per birth (min, max)
    pub litter_size: (u32, u32),
    /// Hunger rate - how fast hunger increases per tick
    pub hunger_rate: f32,
    /// Max hunger before starvation damage begins
    pub max_hunger: f32,
    /// Food value when eaten (for prey animals)
    pub food_value: f32,
    /// Prey species IDs this carnivore/omnivore can hunt
    pub prey_species: Vec<String>,

    // === MIGRATION FIELDS ===
    /// Whether this species migrates seasonally
    pub is_migratory: bool,
    /// Preferred migration direction (dx, dy) per season change
    pub migration_direction: (i32, i32),
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
        // Lifecycle
        lifespan: (8000, 12000),      // Short-lived
        maturity_age: 500,             // Mature quickly
        breeding_cooldown: 300,        // Breed often
        gestation_period: 200,         // Quick gestation
        litter_size: (3, 8),           // Large litters
        hunger_rate: 0.15,             // High metabolism
        max_hunger: 100.0,
        food_value: 15.0,              // Small prey
        prey_species: vec![],          // Herbivore
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (6000, 10000),
        maturity_age: 400,
        breeding_cooldown: 500,
        gestation_period: 250,
        litter_size: (2, 5),
        hunger_rate: 0.12,
        max_hunger: 80.0,
        food_value: 10.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (5000, 8000),
        maturity_age: 300,
        breeding_cooldown: 200,
        gestation_period: 0, // Egg layer
        litter_size: (1, 1), // Eggs handled separately
        hunger_rate: 0.1,
        max_hunger: 80.0,
        food_value: 12.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (10000, 15000),
        maturity_age: 800,
        breeding_cooldown: 1000,
        gestation_period: 400,
        litter_size: (2, 5),
        hunger_rate: 0.08,
        max_hunger: 150.0,
        food_value: 30.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string(), "chicken".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
        can_domesticate: false,
        living_products: vec![],
        lifespan: (12000, 18000),
        maturity_age: 1000,
        breeding_cooldown: 1500,
        gestation_period: 500,
        litter_size: (3, 6),
        hunger_rate: 0.06,
        max_hunger: 200.0,
        food_value: 45.0,
        prey_species: vec!["rabbit".to_string(), "deer".to_string(), "sheep".to_string(), "goat".to_string()],
        is_migratory: true, // Wolves follow prey herds
        migration_direction: (0, -20), // Move south in winter
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
            AnimalDrop::new("antler".to_string(), 2, 2).with_chance(0.5),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (15000, 25000),
        maturity_age: 1200,
        breeding_cooldown: 2000,
        gestation_period: 800,
        litter_size: (1, 2),
        hunger_rate: 0.05,
        max_hunger: 200.0,
        food_value: 60.0,
        prey_species: vec![],
        is_migratory: true, // Deer migrate seasonally
        migration_direction: (0, -15), // Move south in winter
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
                production_time: 600,
                quantity: 4,
            },
        ],
        lifespan: (12000, 18000),
        maturity_age: 800,
        breeding_cooldown: 1500,
        gestation_period: 600,
        litter_size: (1, 3),
        hunger_rate: 0.04,
        max_hunger: 180.0,
        food_value: 50.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (14000, 20000),
        maturity_age: 900,
        breeding_cooldown: 1200,
        gestation_period: 550,
        litter_size: (1, 3),
        hunger_rate: 0.045,
        max_hunger: 170.0,
        food_value: 55.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        can_domesticate: true,
        living_products: vec![],
        lifespan: (12000, 18000),
        maturity_age: 1000,
        breeding_cooldown: 1500,
        gestation_period: 500,
        litter_size: (4, 8),
        hunger_rate: 0.06,
        max_hunger: 220.0,
        food_value: 80.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string()], // Omnivore
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (18000, 28000),
        maturity_age: 1500,
        breeding_cooldown: 2500,
        gestation_period: 900,
        litter_size: (1, 1),
        hunger_rate: 0.04,
        max_hunger: 300.0,
        food_value: 100.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (25000, 35000),
        maturity_age: 2000,
        breeding_cooldown: 4000,
        gestation_period: 800,
        litter_size: (1, 3),
        hunger_rate: 0.03,
        max_hunger: 400.0,
        food_value: 200.0,
        prey_species: vec!["deer".to_string(), "sheep".to_string(), "boar".to_string(), "fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (20000, 30000),
        maturity_age: 1800,
        breeding_cooldown: 3000,
        gestation_period: 700,
        litter_size: (1, 4),
        hunger_rate: 0.035,
        max_hunger: 350.0,
        food_value: 180.0,
        prey_species: vec!["deer".to_string(), "goat".to_string(), "camel".to_string(), "boar".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
            AnimalDrop::new("fur".to_string(), 3, 5),
            AnimalDrop::new("leather".to_string(), 1, 2),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (8000, 12000),
        maturity_age: 600,
        breeding_cooldown: 800,
        gestation_period: 350,
        litter_size: (3, 8),
        hunger_rate: 0.09,
        max_hunger: 140.0,
        food_value: 35.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string(), "fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
            AnimalDrop::new("fur".to_string(), 4, 6),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "milk".to_string(),
                production_time: 300,
                quantity: 1,
            },
        ],
        lifespan: (30000, 50000),
        maturity_age: 2500,
        breeding_cooldown: 4000,
        gestation_period: 1000,
        litter_size: (1, 1),
        hunger_rate: 0.02, // Low metabolism - desert adapted
        max_hunger: 400.0,
        food_value: 120.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (50000, 80000),
        maturity_age: 5000,
        breeding_cooldown: 8000,
        gestation_period: 2000,
        litter_size: (1, 1),
        hunger_rate: 0.025,
        max_hunger: 600.0,
        food_value: 300.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (5000, 8000),
        maturity_age: 300,
        breeding_cooldown: 200,
        gestation_period: 0,
        litter_size: (1, 1),
        hunger_rate: 0.1,
        max_hunger: 70.0,
        food_value: 10.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
            AnimalDrop::new("feathers".to_string(), 6, 10),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "egg".to_string(),
                production_time: 150,
                quantity: 1,
            },
        ],
        lifespan: (6000, 10000),
        maturity_age: 350,
        breeding_cooldown: 250,
        gestation_period: 0,
        litter_size: (1, 1),
        hunger_rate: 0.08,
        max_hunger: 90.0,
        food_value: 15.0,
        prey_species: vec![],
        is_migratory: true, // Geese are classic migratory birds
        migration_direction: (0, -30), // Fly far south in winter
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
        lifespan: (12000, 18000),
        maturity_age: 800,
        breeding_cooldown: 1200,
        gestation_period: 400,
        litter_size: (6, 12),
        hunger_rate: 0.07,
        max_hunger: 200.0,
        food_value: 60.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        speed: 2.0,
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
        lifespan: (10000, 15000),
        maturity_age: 400,
        breeding_cooldown: 500,
        gestation_period: 0,
        litter_size: (3, 6),
        hunger_rate: 0.12,
        max_hunger: 60.0,
        food_value: 8.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        speed: 2.5,
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
        lifespan: (20000, 35000),
        maturity_age: 1500,
        breeding_cooldown: 3000,
        gestation_period: 0,
        litter_size: (1, 3),
        hunger_rate: 0.06,
        max_hunger: 120.0,
        food_value: 25.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string(), "fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
        can_domesticate: true,
        living_products: vec![],
        lifespan: (15000, 25000),
        maturity_age: 1000,
        breeding_cooldown: 2000,
        gestation_period: 0,
        litter_size: (2, 4),
        hunger_rate: 0.07,
        max_hunger: 100.0,
        food_value: 20.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (12000, 20000),
        maturity_age: 800,
        breeding_cooldown: 1500,
        gestation_period: 0,
        litter_size: (2, 5),
        hunger_rate: 0.08,
        max_hunger: 90.0,
        food_value: 18.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
        can_domesticate: true,
        living_products: vec![],
        lifespan: (30000, 60000), // Parrots live very long
        maturity_age: 1500,
        breeding_cooldown: 2000,
        gestation_period: 0,
        litter_size: (2, 4),
        hunger_rate: 0.09,
        max_hunger: 80.0,
        food_value: 12.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        attack_damage: 15.0,
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
        lifespan: (15000, 25000),
        maturity_age: 1000,
        breeding_cooldown: 2000,
        gestation_period: 0, // Egg layer
        litter_size: (5, 20),
        hunger_rate: 0.02, // Very low - can go long without eating
        max_hunger: 200.0,
        food_value: 20.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
        lifespan: (18000, 28000),
        maturity_age: 2000,
        breeding_cooldown: 4000,
        gestation_period: 700,
        litter_size: (2, 4),
        hunger_rate: 0.04,
        max_hunger: 380.0,
        food_value: 190.0,
        prey_species: vec!["deer".to_string(), "boar".to_string(), "goat".to_string(), "monkey".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn crocodile() -> AnimalSpecies {
    AnimalSpecies {
        id: "crocodile".to_string(),
        name: "Crocodile".to_string(),
        description: "Ancient reptilian predator, lurks in water".to_string(),
        health: 150.0,
        attack_damage: 35.0,
        defense: 12.0,
        speed: 0.9,
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
        lifespan: (50000, 80000), // Crocodiles live very long
        maturity_age: 3000,
        breeding_cooldown: 5000,
        gestation_period: 0, // Egg layer
        litter_size: (20, 50),
        hunger_rate: 0.015, // Very low metabolism
        max_hunger: 500.0,
        food_value: 150.0,
        prey_species: vec!["deer".to_string(), "goat".to_string(), "fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
            AnimalDrop::new("fur".to_string(), 15, 20),
            AnimalDrop::new("thick_hide".to_string(), 10, 15),
            AnimalDrop::new("bear_claw".to_string(), 4, 4),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (25000, 40000),
        maturity_age: 2500,
        breeding_cooldown: 5000,
        gestation_period: 900,
        litter_size: (1, 3),
        hunger_rate: 0.025,
        max_hunger: 450.0,
        food_value: 220.0,
        prey_species: vec!["seal".to_string(), "fish".to_string(), "reindeer".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
        can_domesticate: true,
        living_products: vec![],
        lifespan: (18000, 28000),
        maturity_age: 1500,
        breeding_cooldown: 2500,
        gestation_period: 850,
        litter_size: (1, 2),
        hunger_rate: 0.045,
        max_hunger: 250.0,
        food_value: 90.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        can_domesticate: true,
        living_products: vec![],
        lifespan: (15000, 22000),
        maturity_age: 1200,
        breeding_cooldown: 2000,
        gestation_period: 750,
        litter_size: (1, 1),
        hunger_rate: 0.05,
        max_hunger: 200.0,
        food_value: 70.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        can_domesticate: true,
        living_products: vec![],
        lifespan: (20000, 35000),
        maturity_age: 1500,
        breeding_cooldown: 2000,
        gestation_period: 500,
        litter_size: (1, 2),
        hunger_rate: 0.1,
        max_hunger: 120.0,
        food_value: 25.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
        diet: DietType::Carnivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (10, 50),
        drops: vec![
            AnimalDrop::new("fish_meat".to_string(), 1, 2),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (3000, 8000),
        maturity_age: 200,
        breeding_cooldown: 100,
        gestation_period: 0, // Spawn eggs
        litter_size: (50, 200), // Many eggs
        hunger_rate: 0.05,
        max_hunger: 50.0,
        food_value: 5.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
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
            AnimalDrop::new("fur".to_string(), 3, 5),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (12000, 18000),
        maturity_age: 800,
        breeding_cooldown: 1500,
        gestation_period: 400,
        litter_size: (1, 4),
        hunger_rate: 0.1,
        max_hunger: 130.0,
        food_value: 30.0,
        prey_species: vec!["fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
        speed: 1.1,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Carnivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (3, 12),
        drops: vec![
            AnimalDrop::new("seal_meat".to_string(), 10, 15),
            AnimalDrop::new("blubber".to_string(), 8, 12),
            AnimalDrop::new("fur".to_string(), 4, 6),
            AnimalDrop::new("leather".to_string(), 3, 5),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (25000, 40000),
        maturity_age: 2000,
        breeding_cooldown: 3000,
        gestation_period: 800,
        litter_size: (1, 1),
        hunger_rate: 0.04,
        max_hunger: 250.0,
        food_value: 80.0,
        prey_species: vec!["fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
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
    pub max_lifespan: u32, // Maximum age before death

    /// Domestication
    pub is_domesticated: bool,
    pub tame_level: f32, // 0.0 = wild, 1.0 = fully tamed
    pub owner_id: Option<Uuid>, // Agent who owns this animal

    /// Reproduction
    pub can_reproduce: bool,
    pub reproduction_cooldown: u32,
    pub is_pregnant: bool,
    pub pregnancy_timer: u32, // Ticks until birth
    pub mate_id: Option<Uuid>, // For tracking lineage

    /// Hunger/feeding system
    pub hunger: f32,        // Current hunger level (0 = full, max = starving)
    pub max_hunger: f32,    // Max hunger before starvation damage
    pub hunger_rate: f32,   // Hunger increase per tick
    pub is_starving: bool,  // Taking starvation damage

    /// Living product timers
    pub product_timers: HashMap<String, u32>, // material_id -> ticks until production
}

impl Animal {
    pub fn new(species_id: String, position: (i32, i32), species: &AnimalSpecies) -> Self {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        let mut product_timers = HashMap::new();
        for product in &species.living_products {
            product_timers.insert(product.material_id.clone(), product.production_time);
        }

        // Calculate random lifespan within species range
        let max_lifespan = rng.gen_range(species.lifespan.0..=species.lifespan.1);

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
            maturity_age: species.maturity_age,
            max_lifespan,
            is_domesticated: false,
            tame_level: 0.0,
            owner_id: None,
            can_reproduce: true,
            reproduction_cooldown: 0,
            is_pregnant: false,
            pregnancy_timer: 0,
            mate_id: None,
            hunger: 0.0,
            max_hunger: species.max_hunger,
            hunger_rate: species.hunger_rate,
            is_starving: false,
            product_timers,
        }
    }

    /// Create a newborn animal (starts at age 0, inherits some traits)
    pub fn new_offspring(species_id: String, position: (i32, i32), species: &AnimalSpecies, parent_group: Option<Uuid>) -> Self {
        let mut offspring = Self::new(species_id, position, species);
        offspring.group_id = parent_group;
        // Newborns start with some hunger
        offspring.hunger = offspring.max_hunger * 0.3;
        offspring
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

        // Update pregnancy timer
        if self.is_pregnant && self.pregnancy_timer > 0 {
            self.pregnancy_timer -= 1;
        }
    }

    /// Check if animal has died of old age
    pub fn is_too_old(&self) -> bool {
        self.age >= self.max_lifespan
    }

    /// Check if animal can breed (mature, not pregnant, cooldown expired)
    pub fn can_breed(&self) -> bool {
        self.is_alive()
            && self.is_mature()
            && self.can_reproduce
            && !self.is_pregnant
            && self.reproduction_cooldown == 0
            && !self.is_starving
            // Well fed, not merely coping. At seven tenths of everything it
            // can hold an animal still counts as fit to breed, which let both
            // herds and packs go on multiplying through the lean stretch that
            // should have stopped them.
            && self.hunger < self.max_hunger * 0.4
    }

    /// Start pregnancy with gestation period
    pub fn become_pregnant(&mut self, gestation_period: u32, breeding_cooldown: u32) {
        self.is_pregnant = true;
        self.pregnancy_timer = gestation_period;
        self.reproduction_cooldown = breeding_cooldown;
    }

    /// Check if ready to give birth
    pub fn ready_to_give_birth(&self) -> bool {
        self.is_pregnant && self.pregnancy_timer == 0
    }

    /// Complete birth and reset pregnancy state
    pub fn give_birth(&mut self) {
        self.is_pregnant = false;
        self.pregnancy_timer = 0;
    }

    /// Increase hunger by the animal's hunger rate
    pub fn tick_hunger(&mut self) {
        if !self.is_alive() {
            return;
        }

        self.hunger = (self.hunger + self.hunger_rate).min(self.max_hunger * 1.5);

        // Check starvation threshold
        if self.hunger >= self.max_hunger {
            self.is_starving = true;
            // Take starvation damage proportional to how hungry
            let starvation_damage = (self.hunger - self.max_hunger) * 0.1;
            self.take_damage(starvation_damage);
        } else {
            self.is_starving = false;
        }
    }

    /// Feed the animal, reducing hunger
    pub fn feed(&mut self, amount: f32) {
        self.hunger = (self.hunger - amount).max(0.0);
        if self.hunger < self.max_hunger {
            self.is_starving = false;
        }
    }

    /// Check if animal is hungry enough to seek food
    pub fn is_hungry(&self) -> bool {
        self.hunger > self.max_hunger * 0.5
    }

    /// Check if animal is very hungry (urgent food seeking)
    pub fn is_very_hungry(&self) -> bool {
        self.hunger > self.max_hunger * 0.8
    }

    /// Whether a predator will bother going after something it sees.
    ///
    /// Much lower than `is_hungry`, which is half of everything the animal can
    /// hold. A predator that only hunted when it was half starved killed about
    /// one animal in a thousand ticks - far below what the herds breed - so it
    /// stayed hungry, never bred, and the herbivores it was supposed to be
    /// holding down ran to the population cap. A predator that is not nearly
    /// full will take what is in front of it.
    pub fn will_hunt(&self) -> bool {
        self.hunger > self.max_hunger * 0.15
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

    /// The most of each species this world has ever held, which is what a
    /// depleted population is judged against
    #[serde(default)]
    peak_population: HashMap<String, u32>,

    /// Size of the map, so animals wandering in from outside know where the
    /// edge is
    #[serde(default)]
    world_bounds: Option<(i32, i32)>,

    /// Ticks since the last time anything was allowed to wander in
    #[serde(default)]
    ticks_since_migration: u32,

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
            peak_population: HashMap::new(),
            world_bounds: None,
            ticks_since_migration: 0,
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

    /// All animals, mutably
    pub fn get_all_mut(&mut self) -> &mut Vec<Animal> {
        &mut self.animals
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



    /// Get species from registry
    pub fn get_species(&self, species_id: &str) -> Option<&AnimalSpecies> {
        self.registry.as_ref()?.get(species_id)
    }





    /// Tick all animals (age, products, natural healing, AI behaviors, lifecycle)
    pub fn tick(&mut self) {
        if self.registry.is_none() {
            return;
        }

        // First pass: basic updates and lifecycle
        let mut deaths_from_age = Vec::new();
        for (idx, animal) in self.animals.iter_mut().enumerate() {
            if !animal.is_alive() {
                continue;
            }

            // Age
            animal.tick_age();

            // Check for death from old age
            if animal.is_too_old() {
                deaths_from_age.push(idx);
                continue;
            }

            // Hunger system
            animal.tick_hunger();

            // Natural stamina recovery when resting
            if animal.state == AnimalState::Resting {
                animal.recover_stamina(1.0);
            } else if animal.state != AnimalState::Dead {
                // Gradual stamina consumption for active animals
                animal.use_stamina(0.1);
            }

            // Slow natural healing (if not starving)
            if animal.current_health < animal.max_health && !animal.is_starving {
                animal.heal(0.1);
            }

            // Tick products
            animal.tick_products();

            // Decrement state timer
            if animal.state_timer > 0 {
                animal.state_timer -= 1;
            }
        }

        // Kill animals that died of old age
        for idx in deaths_from_age.iter().rev() {
            if let Some(animal) = self.animals.get_mut(*idx) {
                animal.state = AnimalState::Dead;
                animal.current_health = 0.0;
            }
        }

        // Second pass: Births (process pregnant animals ready to give birth)
        self.process_births();

        // Third pass: Breeding attempts
        self.process_breeding();

        // Fourth pass: Predator hunting
        self.process_predation();

        // Animals from beyond the edge of the map, for species that have been
        // wiped out or hunted down to nothing here
        self.process_immigration();

        // Fifth pass: Herbivore feeding (grazing reduces hunger)
        self.process_grazing();

        // Sixth pass: AI behavior (needs fresh registry borrow)
        let animals_data: Vec<(usize, String, AnimalBehavior, bool, bool)> = {
            let registry = match &self.registry {
                Some(r) => r,
                None => return,
            };
            self.animals
                .iter()
                .enumerate()
                .filter(|(_, a)| a.is_alive())
                .filter_map(|(idx, a)| {
                    let species = registry.get(&a.species_id)?;
                    Some((idx, a.species_id.clone(), species.behavior, a.is_wild(), a.is_hungry()))
                })
                .collect()
        };

        for (idx, _species_id, behavior, is_wild, is_hungry) in animals_data {
            self.update_animal_behavior_with_hunger(idx, behavior, is_wild, is_hungry);
        }
    }

    /// What a wild animal does about people.
    ///
    /// Nothing, until now. There was predator hunting in this module and no
    /// other awareness of agents at all, so a deer stood where it stood while
    /// a settlement walked up to it - which is what made a stone-age hunt a
    /// matter of finding an animal rather than of stalking one, and is half of
    /// why food was too easy to come by.
    ///
    /// Most things that live in a wood get out of a person's way. The ones
    /// that do not are the ones that mean to do something about the person:
    /// an aggressive or territorial beast holds its ground, and a tame one
    /// has no reason to run.
    ///
    /// Takes bare positions rather than agents, so that nothing in here has to
    /// know what an agent is.
    pub fn shy_away_from(&mut self, people: &[(i32, i32)]) {
        if people.is_empty() {
            return;
        }

        // Which of them would move off, worked out before the animals are
        // borrowed to be moved. The registry lives beside them and cannot be
        // read while they are held mutably, which is the same dance the AI
        // pass does.
        let skittish: Vec<usize> = {
            let Some(registry) = &self.registry else {
                return;
            };

            self.animals
                .iter()
                .enumerate()
                .filter(|(_, animal)| animal.is_alive() && animal.is_wild())
                .filter(|(_, animal)| {
                    registry.get(&animal.species_id).is_some_and(|species| {
                        matches!(
                            species.behavior,
                            AnimalBehavior::Passive
                                | AnimalBehavior::Neutral
                                | AnimalBehavior::Defensive
                        )
                    })
                })
                .map(|(idx, _)| idx)
                .collect()
        };

        for idx in skittish {
            let animal = &mut self.animals[idx];

            // The nearest person, and only if they are near enough to have
            // been noticed
            let Some(nearest) = people
                .iter()
                .min_by_key(|(x, y)| {
                    (x - animal.position.0).abs().max((y - animal.position.1).abs())
                })
            else {
                continue;
            };

            let how_close = (nearest.0 - animal.position.0)
                .abs()
                .max((nearest.1 - animal.position.1).abs());

            if how_close > Self::NEAR_ENOUGH_TO_SPOOK_IT || how_close == 0 {
                continue;
            }

            // One step directly away. A stone-age hunter is faster over a
            // short dash than a deer is over a long one, which is why hunting
            // works at all; what this does is make him spend the dash.
            let away = |them: i32, it: i32| -> i32 {
                match it.cmp(&them) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Greater => 1,
                    std::cmp::Ordering::Equal => 0,
                }
            };

            animal.position.0 += away(nearest.0, animal.position.0);
            animal.position.1 += away(nearest.1, animal.position.1);
            animal.use_stamina(0.2);
        }
    }

    /// How near somebody has to be before a wild animal thinks better of
    /// standing there.
    ///
    /// A little further than a man can throw, so that walking up to a deer
    /// costs something even when the throw itself would have been easy.
    const NEAR_ENOUGH_TO_SPOOK_IT: i32 = 4;

    /// Process births for pregnant animals
    fn process_births(&mut self) {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        let registry = match &self.registry {
            Some(r) => r,
            None => return,
        };

        // Collect birth data
        let births: Vec<(String, (i32, i32), Option<Uuid>, u32, u32)> = self.animals
            .iter_mut()
            .filter(|a| a.is_alive() && a.ready_to_give_birth())
            .filter_map(|a| {
                let species = registry.get(&a.species_id)?;
                let litter_size = rng.gen_range(species.litter_size.0..=species.litter_size.1);

                // Complete birth
                a.give_birth();

                Some((
                    a.species_id.clone(),
                    a.position,
                    a.group_id,
                    litter_size,
                    ((species.breeding_cooldown as f32) * BREEDING_INTERVAL_SCALE) as u32,
                ))
            })
            .collect();

        // Spawn offspring
        for (species_id, position, group_id, litter_size, _cooldown) in births {
            if let Some(species) = registry.get(&species_id) {
                for _ in 0..litter_size {
                    if self.animals.len() >= self.max_population {
                        break;
                    }

                    // Spawn near parent with some offset
                    let offset_x = rng.gen_range(-2..=2);
                    let offset_y = rng.gen_range(-2..=2);
                    let offspring_pos = (position.0 + offset_x, position.1 + offset_y);

                    let offspring = Animal::new_offspring(
                        species_id.clone(),
                        offspring_pos,
                        species,
                        group_id,
                    );
                    self.animals.push(offspring);
                }
            }
        }
    }

    /// Process breeding attempts for eligible animals
    fn process_breeding(&mut self) {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        let registry = match &self.registry {
            Some(r) => r,
            None => return,
        };

        // Only attempt breeding occasionally (not every tick)
        if rng.gen::<f32>() > 0.01 {
            return;
        }

        // How many mouths are already on each patch of ground.
        //
        // Animals breed when the land around them will carry another. Without
        // this a herd has no size it stops at: grazing feeds every animal
        // nearly a hundred times what it burns, so hunger never becomes the
        // limit, and herds grew until they hit the hard population cap however
        // little ground they were on.
        let mut crowding: HashMap<(i32, i32), u32> = HashMap::new();
        for animal in &self.animals {
            if animal.is_alive() {
                *crowding
                    .entry((animal.position.0 / GRAZING_PATCH, animal.position.1 / GRAZING_PATCH))
                    .or_insert(0) += 1;
            }
        }

        // Find breeding candidates by species
        let mut breeding_candidates: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, animal) in self.animals.iter().enumerate() {
            if !animal.can_breed() {
                continue;
            }

            let patch = (
                animal.position.0 / GRAZING_PATCH,
                animal.position.1 / GRAZING_PATCH,
            );
            if crowding.get(&patch).copied().unwrap_or(0) > PATCH_CARRYING_CAPACITY {
                continue;
            }

            breeding_candidates
                .entry(animal.species_id.clone())
                .or_insert_with(Vec::new)
                .push(idx);
        }

        // For each species with 2+ candidates, attempt breeding
        for (species_id, candidates) in breeding_candidates {
            if candidates.len() < 2 {
                continue;
            }

            // Get species data
            let species = match registry.get(&species_id) {
                Some(s) => s,
                None => continue,
            };

            // Check pairs for proximity (within breeding distance)
            for i in 0..candidates.len() {
                for j in (i + 1)..candidates.len() {
                    let idx_a = candidates[i];
                    let idx_b = candidates[j];

                    let pos_a = self.animals[idx_a].position;
                    let pos_b = self.animals[idx_b].position;

                    // Check proximity. Ten tiles rather than five because
                    // nothing in the model keeps a group together: animals
                    // spawn as a herd and then wander off on their own. A
                    // herd of ten still has pairs in range after that; a pair
                    // of wolves does not, which left predators unable to
                    // breed at all while the herbivores they were supposed to
                    // be holding down ran to the population cap.
                    let distance = ((pos_a.0 - pos_b.0).abs() + (pos_a.1 - pos_b.1).abs()) as f32;
                    if distance <= 10.0 {
                        // Breeding chance based on proximity
                        if rng.gen::<f32>() < 0.3 {
                            // One becomes pregnant (or both go on cooldown for egg-layers)
                            if species.gestation_period > 0 {
                                // Mammal-style: one becomes pregnant
                                self.animals[idx_a].become_pregnant(
                                    species.gestation_period,
                                    ((species.breeding_cooldown as f32) * BREEDING_INTERVAL_SCALE) as u32,
                                );
                                self.animals[idx_b].reproduction_cooldown = ((species.breeding_cooldown as f32) * BREEDING_INTERVAL_SCALE) as u32;
                            } else {
                                // Egg-layer: both go on cooldown, eggs spawn immediately
                                self.animals[idx_a].reproduction_cooldown = ((species.breeding_cooldown as f32) * BREEDING_INTERVAL_SCALE) as u32;
                                self.animals[idx_b].reproduction_cooldown = ((species.breeding_cooldown as f32) * BREEDING_INTERVAL_SCALE) as u32;

                                // Spawn eggs (as new animals with age 0)
                                let litter = rng.gen_range(species.litter_size.0..=species.litter_size.1);
                                for _ in 0..litter {
                                    if self.animals.len() >= self.max_population {
                                        break;
                                    }
                                    let pos = self.animals[idx_a].position;
                                    let offspring = Animal::new_offspring(
                                        species_id.clone(),
                                        pos,
                                        species,
                                        self.animals[idx_a].group_id,
                                    );
                                    self.animals.push(offspring);
                                }
                            }
                            break; // Only one breeding per species per tick
                        }
                    }
                }
            }
        }
    }

    /// Process predator hunting - carnivores/omnivores hunt prey
    fn process_predation(&mut self) {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        let registry = match &self.registry {
            Some(r) => r,
            None => return,
        };

        // Every hungry predator hunts on its own account.
        //
        // This used to sit behind a single roll for the whole world - one
        // chance in fifty per tick that any predation happened anywhere - so
        // predators were barely a presence and herbivores grew until they hit
        // the population cap. A predator hunts when it is hungry and not
        // otherwise, which is what ties its numbers to the herds.
        const HUNT_ATTEMPT_CHANCE: f32 = 0.05;

        // Find hungry predators and their prey
        let predator_data: Vec<(usize, String, Vec<String>, (i32, i32), f32, bool, AnimalSize)> =
            self.animals
                .iter()
                .enumerate()
                .filter(|(_, a)| a.is_alive() && a.will_hunt())
                .filter_map(|(idx, a)| {
                    let species = registry.get(&a.species_id)?;
                    if species.prey_species.is_empty() {
                        return None;
                    }
                    // The biggest thing it knows how to bring down, which is
                    // what bounds what it will try when it is desperate. A
                    // wolf takes deer, so a goat is fair game; a fox takes
                    // rabbits, and no amount of hunger makes a cow catchable.
                    let usual_limit = species
                        .prey_species
                        .iter()
                        .filter_map(|prey| registry.get(prey))
                        .map(|prey| prey.size)
                        .max()
                        .unwrap_or(species.size);

                    Some((
                        idx,
                        a.species_id.clone(),
                        species.prey_species.clone(),
                        a.position,
                        species.attack_damage,
                        a.is_very_hungry(),
                        usual_limit,
                    ))
                })
                .collect();

        // For each predator, look for nearby prey
        let mut kills = Vec::new();
        for (pred_idx, _pred_species, prey_species, pred_pos, attack, desperate, usual_limit) in
            predator_data
        {
            if rng.gen::<f32>() > HUNT_ATTEMPT_CHANCE {
                continue;
            }

            // Find nearby prey
            for (prey_idx, prey) in self.animals.iter().enumerate() {
                if !prey.is_alive() || prey_idx == pred_idx {
                    continue;
                }

                // Normally a predator takes what it knows how to take. One
                // that is close to starving takes whatever it can catch: any
                // grazing animal no bigger than itself will do.
                let usual_prey = prey_species.contains(&prey.species_id);
                let worth_trying = usual_prey
                    || (desperate
                        && registry
                            .get(&prey.species_id)
                            .map(|s| s.prey_species.is_empty() && s.size <= usual_limit)
                            .unwrap_or(false));

                if !worth_trying {
                    continue;
                }

                // Check proximity (hunting range of 8 tiles)
                let distance = ((pred_pos.0 - prey.position.0).abs()
                    + (pred_pos.1 - prey.position.1).abs()) as f32;
                if distance > 8.0 {
                    continue;
                }

                // Hunt success based on speed comparison and randomness
                let prey_species_data = registry.get(&prey.species_id);
                let prey_speed = prey_species_data.map(|s| s.speed).unwrap_or(1.0);
                let pred_species_data = registry.get(&self.animals[pred_idx].species_id);
                let pred_speed = pred_species_data.map(|s| s.speed).unwrap_or(1.0);

                let chase_chance = (pred_speed / prey_speed).min(1.0) * 0.4;

                if rng.gen::<f32>() < chase_chance {
                    // Successful hunt - attack prey
                    let prey_food_value = prey_species_data.map(|s| s.food_value).unwrap_or(10.0);
                    kills.push((pred_idx, prey_idx, attack, prey_food_value));
                    break; // One hunt per predator per tick
                }
            }
        }

        // Apply kills
        for (pred_idx, prey_idx, damage, food_value) in kills {
            // Damage prey
            if let Some(prey) = self.animals.get_mut(prey_idx) {
                prey.take_damage(damage);
            }

            // If prey died, feed predator
            if let Some(prey) = self.animals.get(prey_idx) {
                if !prey.is_alive() {
                    if let Some(predator) = self.animals.get_mut(pred_idx) {
                        predator.feed(food_value);
                    }
                }
            }
        }
    }

    /// Animals wander in from beyond the edge of the map.
    ///
    /// A species that has been wiped out here, or hunted down to a quarter of
    /// the most this world ever held of it, is not gone from the world
    /// entirely - only from this corner of it - and a few will find their way
    /// back. Only species that have lived here migrate in: the map does not
    /// invent lions for a valley that never had any.
    ///
    /// Deliberately rare. One small group per depleted species every eight
    /// thousand ticks or so, which is a lifetime for most of them. It is meant
    /// to keep a world from emptying out for good, not to be a larder that
    /// refills itself faster than it can be emptied - a settlement that clears
    /// the herds waits a long time for more.
    fn process_immigration(&mut self) {
        use rand::Rng;

        /// How often anything is allowed to arrive at all
        const MIGRATION_INTERVAL: u32 = 2000;

        /// And how often, at those moments, anything actually does
        const MIGRATION_CHANCE: f64 = 0.25;

        /// Below this share of the most this world ever held, a species counts
        /// as needing help
        const DEPLETED_SHARE: f32 = 0.25;

        /// How many arrive at once
        const ARRIVALS: (u32, u32) = (1, 3);

        self.ticks_since_migration += 1;
        if self.ticks_since_migration < MIGRATION_INTERVAL {
            return;
        }
        self.ticks_since_migration = 0;

        let bounds = match self.world_bounds {
            Some(bounds) => bounds,
            None => return,
        };

        // What is here now, and the most there has ever been
        let mut present: HashMap<String, u32> = HashMap::new();
        for animal in &self.animals {
            if animal.is_alive() {
                *present.entry(animal.species_id.clone()).or_insert(0) += 1;
            }
        }

        for (species_id, count) in &present {
            let peak = self.peak_population.entry(species_id.clone()).or_insert(0);
            *peak = (*peak).max(*count);
        }

        let depleted: Vec<String> = self
            .peak_population
            .iter()
            .filter(|(species_id, peak)| {
                let here = present.get(*species_id).copied().unwrap_or(0) as f32;
                here < (**peak as f32) * DEPLETED_SHARE
            })
            .map(|(species_id, _)| species_id.clone())
            .collect();

        if depleted.is_empty() {
            return;
        }

        let mut rng = crate::core::dice::roll();

        for species_id in depleted {
            if self.animals.len() >= self.max_population {
                break;
            }

            if !rng.gen_bool(MIGRATION_CHANCE) {
                continue;
            }

            // In from one of the four edges
            let arrival = match rng.gen_range(0..4) {
                0 => (rng.gen_range(0..bounds.0), 0),
                1 => (rng.gen_range(0..bounds.0), bounds.1 - 1),
                2 => (0, rng.gen_range(0..bounds.1)),
                _ => (bounds.0 - 1, rng.gen_range(0..bounds.1)),
            };

            let arriving = rng.gen_range(ARRIVALS.0..=ARRIVALS.1);
            self.spawn_group(species_id, arrival, arriving);
        }
    }

    /// Process grazing - herbivores reduce hunger when grazing.
    ///
    /// What a mouthful is worth depends on how many other mouths are on the
    /// same ground. Grazing used to feed every animal the same amount however
    /// many of them there were, which is to say the grass was infinite: nothing
    /// stopped a herd growing until it hit the hard population cap, and no
    /// number of predators could hold a herd that had no other limit on it.
    fn process_grazing(&mut self) {
        /// How many grazers a patch feeds properly before they start
        /// competing for it
        const GRAZERS_PER_PATCH: f32 = 6.0;

        let registry = match &self.registry {
            Some(r) => r,
            None => return,
        };

        // How many grazers are on each patch of ground
        let mut crowding: HashMap<(i32, i32), f32> = HashMap::new();
        for animal in &self.animals {
            if !animal.is_alive() {
                continue;
            }

            let grazes = registry
                .get(&animal.species_id)
                .map(|species| {
                    matches!(species.diet, DietType::Herbivore | DietType::Omnivore)
                })
                .unwrap_or(false);

            if grazes {
                *crowding
                    .entry((
                        animal.position.0 / GRAZING_PATCH,
                        animal.position.1 / GRAZING_PATCH,
                    ))
                    .or_insert(0.0) += 1.0;
            }
        }

        for animal in &mut self.animals {
            if !animal.is_alive() {
                continue;
            }

            // Only grazing animals get food
            if animal.state != AnimalState::Grazing {
                continue;
            }

            // Get diet type
            if let Some(species) = registry.get(&animal.species_id) {
                match species.diet {
                    DietType::Herbivore | DietType::Omnivore => {
                        // Grazing provides food (proportional to size)
                        let graze_amount = match species.size {
                            AnimalSize::Tiny => 2.0,
                            AnimalSize::Small => 4.0,
                            AnimalSize::Medium => 6.0,
                            AnimalSize::Large => 10.0,
                            AnimalSize::Huge => 15.0,
                        };

                        let mouths = crowding
                            .get(&(
                                animal.position.0 / GRAZING_PATCH,
                                animal.position.1 / GRAZING_PATCH,
                            ))
                            .copied()
                            .unwrap_or(1.0);
                        let share = 1.0 / (1.0 + (mouths / GRAZERS_PER_PATCH));

                        animal.feed(graze_amount * share);
                    }
                    DietType::Carnivore => {
                        // Carnivores don't benefit from grazing
                    }
                }
            }
        }
    }

    /// Update animal behavior with hunger consideration
    fn update_animal_behavior_with_hunger(&mut self, animal_idx: usize, behavior: AnimalBehavior, is_wild: bool, is_hungry: bool) {
        let animal = &mut self.animals[animal_idx];

        // If state timer is active, continue current behavior
        if animal.state_timer > 0 {
            return;
        }

        // Hungry animals prioritize food seeking
        if is_hungry && animal.is_very_hungry() {
            animal.state = AnimalState::Grazing; // Or hunting for carnivores
            animal.state_timer = 40;
            // Move while seeking food
            let offset = (rand::random::<i32>() % 5 - 2, rand::random::<i32>() % 5 - 2);
            animal.position.0 += offset.0;
            animal.position.1 += offset.1;
            return;
        }

        // Normal behavior based on type
        match behavior {
            AnimalBehavior::Passive => {
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 50;
                } else if is_hungry || rand::random::<f32>() < 0.3 {
                    animal.state = AnimalState::Grazing;
                    animal.state_timer = 30;
                    let offset = (rand::random::<i32>() % 3 - 1, rand::random::<i32>() % 3 - 1);
                    animal.position.0 += offset.0;
                    animal.position.1 += offset.1;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 20;
                }
            }
            AnimalBehavior::Neutral => {
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 40;
                } else if is_hungry {
                    animal.state = AnimalState::Grazing;
                    animal.state_timer = 35;
                } else if rand::random::<f32>() < 0.2 {
                    animal.state = AnimalState::Drinking;
                    animal.state_timer = 25;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 25;
                }
            }
            AnimalBehavior::Defensive => {
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 45;
                } else if is_hungry || rand::random::<f32>() < 0.5 {
                    animal.state = AnimalState::Grazing;
                    animal.state_timer = 35;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 20;
                }
            }
            AnimalBehavior::Aggressive | AnimalBehavior::Territorial => {
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 60;
                } else if is_wild && (is_hungry || rand::random::<f32>() < 0.3) {
                    animal.state = AnimalState::Hunting { target_id: None };
                    animal.state_timer = 50;
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

    /// Spawn animals naturalistically based on terrain during world generation
    ///
    /// This method spawns animals in appropriate biomes based on terrain types:
    /// - Herbivores spawn in herds in plains, meadows, and forests
    /// - Predators spawn in smaller numbers
    /// - Aquatic animals spawn near water
    /// - Mountain animals spawn in highlands
    pub fn spawn_naturalistic(&mut self, grid: &Grid, config: &AnimalSpawnConfig) {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        let registry = match &self.registry {
            Some(r) => r.clone(),
            None => return,
        };

        self.world_bounds = Some((grid.width as i32, grid.height as i32));

        let total_tiles = grid.width * grid.height;
        let total_herds = (total_tiles * config.herds_per_10000_tiles) / 10000;

        // Categorize species by diet for balanced spawning
        let herbivores: Vec<_> = registry.all_species()
            .into_iter()
            .filter(|s| s.diet == DietType::Herbivore)
            .collect();
        let predators: Vec<_> = registry.all_species()
            .into_iter()
            .filter(|s| s.diet == DietType::Carnivore || s.diet == DietType::Omnivore)
            .filter(|s| !s.prey_species.is_empty())
            .collect();

        if herbivores.is_empty() {
            return;
        }

        // Calculate prey vs predator herds
        let prey_herds = if config.spawn_predators && !predators.is_empty() {
            ((total_herds as f32) * config.prey_to_predator_ratio / (config.prey_to_predator_ratio + 1.0)) as usize
        } else {
            total_herds
        };
        let predator_herds = total_herds.saturating_sub(prey_herds);

        // Collect terrain positions by climate zone
        let mut positions_by_climate: HashMap<ClimateZone, Vec<(i32, i32)>> = HashMap::new();
        for y in 0..grid.height {
            for x in 0..grid.width {
                let terrain = grid.tiles[y][x].terrain.terrain_type;
                // Skip water for land animals
                if terrain == TerrainType::Water {
                    continue;
                }
                let climate = terrain_to_climate_zone(terrain);
                positions_by_climate.entry(climate)
                    .or_insert_with(Vec::new)
                    .push((x as i32, y as i32));
            }
        }

        // Spawn herbivore herds
        let mut spawned = 0;
        let mut prey_present: std::collections::HashSet<String> = std::collections::HashSet::new();
        for _ in 0..prey_herds {
            if spawned >= config.max_initial_population || self.animals.len() >= self.max_population {
                break;
            }

            // Pick a random herbivore species
            let species = &herbivores[rng.gen_range(0..herbivores.len())];

            // Find a position in an appropriate biome
            let climate = if !species.primary_biomes.is_empty() {
                species.primary_biomes[rng.gen_range(0..species.primary_biomes.len())]
            } else {
                ClimateZone::Temperate
            };

            if let Some(positions) = positions_by_climate.get(&climate) {
                if !positions.is_empty() {
                    let pos = positions[rng.gen_range(0..positions.len())];
                    let herd_size = rng.gen_range(species.group_size.0..=species.group_size.1);

                    if let Some(_) = self.spawn_group(species.id.clone(), pos, herd_size) {
                        spawned += herd_size as usize;
                        prey_present.insert(species.id.clone());
                    }
                }
            }
        }

        // Spawn predator groups (smaller).
        //
        // Only predators that eat something living here. Drawing the two lists
        // independently put foxes, which eat rabbits and squirrels, into
        // worlds of sheep and cattle: they never found a meal in eight
        // thousand ticks, their hunger climbed in a straight line from birth
        // to death, and the herds they should have been holding down ran to
        // the population cap unopposed.
        let feedable: Vec<_> = predators
            .iter()
            .filter(|species| {
                species
                    .prey_species
                    .iter()
                    .any(|prey| prey_present.contains(prey))
            })
            .collect();

        if config.spawn_predators && !feedable.is_empty() {
            for _ in 0..predator_herds {
                if spawned >= config.max_initial_population || self.animals.len() >= self.max_population {
                    break;
                }

                // Pick a predator that can live off what is here
                let species = feedable[rng.gen_range(0..feedable.len())];

                // Find a position in an appropriate biome
                let climate = if !species.primary_biomes.is_empty() {
                    species.primary_biomes[rng.gen_range(0..species.primary_biomes.len())]
                } else {
                    ClimateZone::Temperate
                };

                if let Some(positions) = positions_by_climate.get(&climate) {
                    if !positions.is_empty() {
                        let pos = positions[rng.gen_range(0..positions.len())];
                        // Predator packs are typically smaller
                        let pack_size = rng.gen_range(1..=species.group_size.1.min(4));

                        if let Some(_) = self.spawn_group(species.id.clone(), pos, pack_size) {
                            spawned += pack_size as usize;
                        }
                    }
                }
            }
        }
    }

    /// Get a summary of spawned animals by species
    pub fn population_summary(&self) -> HashMap<String, usize> {
        let mut summary = HashMap::new();
        for animal in &self.animals {
            if animal.is_alive() {
                *summary.entry(animal.species_id.clone()).or_insert(0) += 1;
            }
        }
        summary
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

    // ========================================================================
    // LIFECYCLE TESTS
    // ========================================================================

    #[test]
    fn test_animal_aging() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        let initial_age = animal.age;
        animal.tick_age();
        assert_eq!(animal.age, initial_age + 1);
    }

    #[test]
    fn test_animal_death_from_old_age() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        // Set age beyond lifespan
        animal.age = species.lifespan.1 + 100;
        animal.max_lifespan = species.lifespan.1;

        assert!(animal.is_too_old());
    }

    #[test]
    fn test_animal_maturity() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        // Young animal
        animal.age = 10;
        animal.maturity_age = 500;
        assert!(!animal.can_breed());

        // Mature animal with cooldown 0
        animal.age = 600;
        animal.reproduction_cooldown = 0;
        assert!(animal.can_breed());
    }

    #[test]
    fn test_animal_hunger_system() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        let initial_hunger = animal.hunger;
        animal.tick_hunger();
        assert!(animal.hunger > initial_hunger);

        // Feed the animal
        let hunger_before_feed = animal.hunger;
        animal.feed(50.0);
        assert!(animal.hunger < hunger_before_feed);
    }

    #[test]
    fn test_animal_starvation() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        animal.hunger = animal.max_hunger + 10.0;
        animal.tick_hunger();

        assert!(animal.is_starving);
        assert!(animal.current_health < species.health);
    }

    #[test]
    fn test_pregnancy_and_birth() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        // Make animal pregnant
        animal.become_pregnant(100, 1000);
        assert!(animal.is_pregnant);
        assert_eq!(animal.pregnancy_timer, 100);

        // Advance pregnancy
        for _ in 0..100 {
            animal.tick_age(); // This decrements pregnancy timer
        }

        assert!(animal.ready_to_give_birth());

        // Give birth
        animal.give_birth();
        assert!(!animal.is_pregnant);
        assert!(animal.reproduction_cooldown > 0);
    }

    #[test]
    fn test_offspring_creation() {
        let species = rabbit();
        let parent_pos = (10, 20);
        let offspring = Animal::new_offspring(
            "rabbit".to_string(),
            parent_pos,
            &species,
            None,
        );

        assert_eq!(offspring.species_id, "rabbit");
        assert_eq!(offspring.age, 0);
        assert!(!offspring.can_breed()); // Too young
        assert!(offspring.is_alive());
    }

    #[test]
    fn test_predator_prey_species() {
        let wolf = wolf();
        let rabbit = rabbit();

        // Wolf should have prey species
        assert!(!wolf.prey_species.is_empty());
        assert!(wolf.prey_species.contains(&"rabbit".to_string()));

        // Rabbit should not have prey species (herbivore)
        assert!(rabbit.prey_species.is_empty());
    }

    #[test]
    fn test_animal_manager_tick_aging() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("rabbit".to_string(), (0, 0));

        let initial_age = manager.animals[0].age;
        manager.tick();

        // Animal should have aged
        assert!(manager.animals[0].age > initial_age);
    }

    // ========================================================================
    // WHAT A WILD ANIMAL DOES ABOUT PEOPLE
    //
    // Nothing, until ISSUES_FOUND #57. There was predator hunting in this
    // module and no other awareness of agents at all, so a deer stood where it
    // stood while a settlement walked up to it.
    // ========================================================================

    #[test]
    fn a_deer_does_not_stand_still_while_you_walk_up_to_it() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("deer".to_string(), (10, 10));

        let was = manager.animals[0].position;
        manager.shy_away_from(&[(8, 10)]);
        let now = manager.animals[0].position;

        assert_ne!(was, now, "it should have moved");
        assert!(
            (now.0 - 8).abs() > (was.0 - 8).abs(),
            "and moved away from the man, not towards him: {was:?} -> {now:?}"
        );
    }

    #[test]
    fn something_across_the_valley_has_not_noticed_you() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("deer".to_string(), (10, 10));

        let was = manager.animals[0].position;
        manager.shy_away_from(&[(10 + AnimalManager::NEAR_ENOUGH_TO_SPOOK_IT + 1, 10)]);

        assert_eq!(
            manager.animals[0].position, was,
            "a man a long way off is not a reason to move"
        );
    }

    #[test]
    fn a_wolf_does_not_get_out_of_your_way() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("wolf".to_string(), (10, 10));

        let was = manager.animals[0].position;
        manager.shy_away_from(&[(9, 10)]);

        assert_eq!(
            manager.animals[0].position, was,
            "a thing that means to do something about you holds its ground"
        );
    }

    #[test]
    fn an_empty_country_spooks_nothing() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("deer".to_string(), (10, 10));

        let was = manager.animals[0].position;
        manager.shy_away_from(&[]);

        assert_eq!(manager.animals[0].position, was);
    }

    #[test]
    fn test_animal_manager_population_summary() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("rabbit".to_string(), (0, 0));
        manager.spawn_animal("rabbit".to_string(), (1, 0));
        manager.spawn_animal("wolf".to_string(), (5, 5));

        let summary = manager.population_summary();
        assert_eq!(summary.get("rabbit"), Some(&2));
        assert_eq!(summary.get("wolf"), Some(&1));
    }

    #[test]
    fn test_terrain_to_climate_zone() {
        assert_eq!(terrain_to_climate_zone(TerrainType::Desert), ClimateZone::Desert);
        assert_eq!(terrain_to_climate_zone(TerrainType::Mountain), ClimateZone::Arctic);
        assert_eq!(terrain_to_climate_zone(TerrainType::Plains), ClimateZone::Temperate);
        assert_eq!(terrain_to_climate_zone(TerrainType::Forest), ClimateZone::Temperate);
    }

    #[test]
    fn test_animal_spawn_config_default() {
        let config = AnimalSpawnConfig::default();
        assert!(config.herds_per_10000_tiles > 0);
        assert!(config.spawn_predators);
        assert!(config.prey_to_predator_ratio > 1.0);
    }
}
