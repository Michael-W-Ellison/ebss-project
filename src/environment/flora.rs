// src/environment/flora.rs
//! Plant life and vegetation system with biome distributions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Climate zone classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiomeType {
    Arctic,
    Temperate,
    Desert,
    Tropical,
}

/// Growth stage of a plant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrowthStage {
    Seedling,
    Growing,
    Mature,
    Flowering,
    Fruiting,
}

/// A plant species
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantSpecies {
    pub id: String,
    pub name: String,
    pub description: String,

    /// Health/durability when harvesting
    pub health: f32,
    /// How long it takes to grow to maturity (ticks)
    pub growth_time: u32,
    /// Whether it regrows after harvest
    pub regrows: bool,
    /// Time to regrow if applicable
    pub regrow_time: u32,

    /// Primary biomes where this plant thrives
    pub primary_biomes: Vec<BiomeType>,
    /// Secondary biomes where it can grow (lower yield)
    pub secondary_biomes: Vec<BiomeType>,

    /// Materials dropped when harvested
    pub drops: Vec<PlantDrop>,

    /// Whether this is a tree
    pub is_tree: bool,
    /// Size category
    pub size: PlantSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlantSize {
    Tiny,    // Moss, small herbs
    Small,   // Flowers, small bushes
    Medium,  // Large bushes, young trees
    Large,   // Mature trees
    Huge,    // Ancient trees
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantDrop {
    pub material_id: String,
    pub min_quantity: u32,
    pub max_quantity: u32,
    /// Only available at certain growth stages
    pub required_stage: Option<GrowthStage>,
}

impl PlantDrop {
    pub fn new(material_id: String, min: u32, max: u32) -> Self {
        Self {
            material_id,
            min_quantity: min,
            max_quantity: max,
            required_stage: None,
        }
    }

    pub fn at_stage(mut self, stage: GrowthStage) -> Self {
        self.required_stage = Some(stage);
        self
    }
}

/// Plant species database
pub struct FloraRegistry {
    species: HashMap<String, PlantSpecies>,
}

impl FloraRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            species: HashMap::new(),
        };

        registry.register_all_species();
        registry
    }

    fn register(&mut self, species: PlantSpecies) {
        self.species.insert(species.id.clone(), species);
    }

    pub fn get(&self, id: &str) -> Option<&PlantSpecies> {
        self.species.get(id)
    }

    pub fn get_by_biome(&self, biome: BiomeType) -> Vec<&PlantSpecies> {
        self.species
            .values()
            .filter(|s| s.primary_biomes.contains(&biome) || s.secondary_biomes.contains(&biome))
            .collect()
    }

    fn register_all_species(&mut self) {
        // Trees
        self.register(oak_tree());
        self.register(pine_tree());
        self.register(birch_tree());
        self.register(palm_tree());
        self.register(cactus());

        // Fiber plants
        self.register(flax_plant());
        self.register(cotton_plant());
        self.register(hemp_plant());

        // Bushes and shrubs
        self.register(berry_bush());
        self.register(willow_shrub());

        // Grasses and herbs
        self.register(grass());
        self.register(medicinal_herb());
    }
}

// ============================================================================
// TREES
// ============================================================================

fn oak_tree() -> PlantSpecies {
    PlantSpecies {
        id: "oak_tree".to_string(),
        name: "Oak Tree".to_string(),
        description: "Sturdy hardwood tree with strong bark and dense wood".to_string(),
        health: 200.0,
        growth_time: 5000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 15, 25),
            PlantDrop::new("bark".to_string(), 8, 12),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn pine_tree() -> PlantSpecies {
    PlantSpecies {
        id: "pine_tree".to_string(),
        name: "Pine Tree".to_string(),
        description: "Coniferous tree adapted to cold climates, provides resin".to_string(),
        health: 180.0,
        growth_time: 4500,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![BiomeType::Arctic, BiomeType::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 12, 20),
            PlantDrop::new("bark".to_string(), 6, 10),
            PlantDrop::new("resin".to_string(), 2, 4),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn birch_tree() -> PlantSpecies {
    PlantSpecies {
        id: "birch_tree".to_string(),
        name: "Birch Tree".to_string(),
        description: "White-barked tree with flexible branches and medicinal bark".to_string(),
        health: 150.0,
        growth_time: 3500,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![BiomeType::Arctic, BiomeType::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 10, 15),
            PlantDrop::new("bark".to_string(), 10, 15), // More bark than other trees
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn palm_tree() -> PlantSpecies {
    PlantSpecies {
        id: "palm_tree".to_string(),
        name: "Palm Tree".to_string(),
        description: "Tropical tree with fibrous bark and edible fruits".to_string(),
        health: 140.0,
        growth_time: 3000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![BiomeType::Tropical],
        secondary_biomes: vec![BiomeType::Desert],
        drops: vec![
            PlantDrop::new("wood".to_string(), 8, 12),
            PlantDrop::new("plant_fiber".to_string(), 15, 20),
            PlantDrop::new("coconut".to_string(), 3, 6).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn cactus() -> PlantSpecies {
    PlantSpecies {
        id: "cactus".to_string(),
        name: "Cactus".to_string(),
        description: "Water-storing desert plant with thick fibrous interior".to_string(),
        health: 100.0,
        growth_time: 2000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![BiomeType::Desert],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("plant_fiber".to_string(), 8, 12),
            PlantDrop::new("cactus_water".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

// ============================================================================
// FIBER PLANTS
// ============================================================================

fn flax_plant() -> PlantSpecies {
    PlantSpecies {
        id: "flax".to_string(),
        name: "Flax Plant".to_string(),
        description: "Slender plant with blue flowers, produces strong fibers for linen".to_string(),
        health: 20.0,
        growth_time: 180,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("flax_fiber".to_string(), 3, 5),
            PlantDrop::new("flax_seeds".to_string(), 1, 3).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn cotton_plant() -> PlantSpecies {
    PlantSpecies {
        id: "cotton".to_string(),
        name: "Cotton Plant".to_string(),
        description: "Produces soft, fluffy fibers ideal for comfortable clothing".to_string(),
        health: 25.0,
        growth_time: 200,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![BiomeType::Temperate, BiomeType::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("cotton".to_string(), 4, 6).at_stage(GrowthStage::Fruiting),
            PlantDrop::new("cotton_seeds".to_string(), 2, 4).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn hemp_plant() -> PlantSpecies {
    PlantSpecies {
        id: "hemp".to_string(),
        name: "Hemp Plant".to_string(),
        description: "Hardy plant with strong fibers, grows quickly".to_string(),
        health: 30.0,
        growth_time: 150,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Tropical],
        drops: vec![
            PlantDrop::new("plant_fiber".to_string(), 5, 8),
            PlantDrop::new("hemp_seeds".to_string(), 2, 4).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

// ============================================================================
// BUSHES AND SHRUBS
// ============================================================================

fn berry_bush() -> PlantSpecies {
    PlantSpecies {
        id: "berry_bush".to_string(),
        name: "Berry Bush".to_string(),
        description: "Thorny bush producing edible berries".to_string(),
        health: 40.0,
        growth_time: 500,
        regrows: true,
        regrow_time: 300,
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Tropical],
        drops: vec![
            PlantDrop::new("berries".to_string(), 5, 10).at_stage(GrowthStage::Fruiting),
            PlantDrop::new("plant_fiber".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

fn willow_shrub() -> PlantSpecies {
    PlantSpecies {
        id: "willow_shrub".to_string(),
        name: "Willow Shrub".to_string(),
        description: "Flexible branches useful for weaving and crafting".to_string(),
        health: 35.0,
        growth_time: 400,
        regrows: true,
        regrow_time: 250,
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Arctic],
        drops: vec![
            PlantDrop::new("willow_branches".to_string(), 3, 6),
            PlantDrop::new("plant_fiber".to_string(), 2, 3),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

// ============================================================================
// GRASSES AND HERBS
// ============================================================================

fn grass() -> PlantSpecies {
    PlantSpecies {
        id: "grass".to_string(),
        name: "Grass".to_string(),
        description: "Common ground cover providing basic plant fibers".to_string(),
        health: 5.0,
        growth_time: 50,
        regrows: true,
        regrow_time: 30,
        primary_biomes: vec![BiomeType::Temperate, BiomeType::Tropical],
        secondary_biomes: vec![BiomeType::Desert],
        drops: vec![
            PlantDrop::new("plant_fiber".to_string(), 1, 2),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn medicinal_herb() -> PlantSpecies {
    PlantSpecies {
        id: "medicinal_herb".to_string(),
        name: "Medicinal Herb".to_string(),
        description: "Aromatic plant with healing properties".to_string(),
        health: 10.0,
        growth_time: 120,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![BiomeType::Temperate],
        secondary_biomes: vec![BiomeType::Tropical],
        drops: vec![
            PlantDrop::new("medicinal_herbs".to_string(), 2, 4).at_stage(GrowthStage::Flowering),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flora_registry() {
        let registry = FloraRegistry::new();

        // Should have all species registered
        assert!(registry.get("oak_tree").is_some());
        assert!(registry.get("flax").is_some());
        assert!(registry.get("cotton").is_some());
    }

    #[test]
    fn test_biome_filtering() {
        let registry = FloraRegistry::new();

        let temperate_plants = registry.get_by_biome(BiomeType::Temperate);
        assert!(!temperate_plants.is_empty());

        // Oak should be in temperate
        assert!(temperate_plants.iter().any(|p| p.id == "oak_tree"));
    }

    #[test]
    fn test_plant_drops() {
        let flax = flax_plant();
        assert!(!flax.drops.is_empty());

        // Should drop flax fiber
        assert!(flax.drops.iter().any(|d| d.material_id == "flax_fiber"));
    }

    #[test]
    fn test_trees_vs_plants() {
        let oak = oak_tree();
        let flax = flax_plant();

        assert!(oak.is_tree);
        assert!(!flax.is_tree);
        assert_eq!(oak.size, PlantSize::Large);
        assert_eq!(flax.size, PlantSize::Small);
    }

    #[test]
    fn test_regrowth() {
        let grass = grass();
        let oak = oak_tree();

        assert!(grass.regrows);
        assert!(!oak.regrows);
    }
}
