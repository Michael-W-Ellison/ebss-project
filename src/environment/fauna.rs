// src/environment/fauna.rs
//! Animal life and wildlife system with biome distributions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::flora::BiomeType;

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
    pub primary_biomes: Vec<BiomeType>,
    pub secondary_biomes: Vec<BiomeType>,
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

    pub fn get_by_biome(&self, biome: BiomeType) -> Vec<&AnimalSpecies> {
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

        // Small animals
        self.register(fox());
        self.register(wolf());

        // Medium herbivores (domesticable)
        self.register(deer());
        self.register(sheep());
        self.register(goat());

        // Medium/Large omnivores
        self.register(boar());
        self.register(cow());

        // Large predators
        self.register(bear());
        self.register(lion());

        // Arctic/Desert specialists
        self.register(arctic_fox());
        self.register(camel());
        self.register(mammoth());
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
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Desert],
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
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Arctic],
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
        primary_biomes: vec![BiomeType::Temperate, BiomeType::Tropical],
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
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Arctic],
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
        primary_biomes: vec![BiomeType::Temperate, BiomeType::Arctic],
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
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Arctic],
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
        primary_biomes: vec![BiomeType::Temperate],
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
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Desert],
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
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Tropical],
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
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Tropical],
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
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Arctic],
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
        primary_biomes: vec![BiomeType::Desert],
        secondary_biomes: vec![BiomeType::Tropical],
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
        primary_biomes: vec![BiomeType::Arctic],
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
        primary_biomes: vec![BiomeType::Desert],
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
        primary_biomes: vec![BiomeType::Arctic],
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

        let arctic_animals = registry.get_by_biome(BiomeType::Arctic);
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
