// src/environment/flora.rs
//! Plant life and vegetation system with biome distributions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Climate zone classification (broad categories)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClimateZone {
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
    pub primary_biomes: Vec<ClimateZone>,
    /// Secondary biomes where it can grow (lower yield)
    pub secondary_biomes: Vec<ClimateZone>,

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
#[derive(Debug, Clone)]
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

    pub fn get_by_biome(&self, biome: ClimateZone) -> Vec<&PlantSpecies> {
        self.species
            .values()
            .filter(|s| s.primary_biomes.contains(&biome) || s.secondary_biomes.contains(&biome))
            .collect()
    }

    pub fn all_species(&self) -> Vec<&PlantSpecies> {
        self.species.values().collect()
    }

    pub fn get_trees(&self) -> Vec<&PlantSpecies> {
        self.species.values().filter(|s| s.is_tree).collect()
    }

    pub fn get_crops(&self) -> Vec<&PlantSpecies> {
        self.species
            .values()
            .filter(|s| s.id.contains("wheat") || s.id.contains("barley") ||
                       s.id.contains("corn") || s.id.contains("rice") ||
                       s.id.contains("potato") || s.id.contains("carrot") ||
                       s.id.contains("onion") || s.id.contains("cabbage") ||
                       s.id.contains("tomato"))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.species.len()
    }

    fn register_all_species(&mut self) {
        // Trees (Basic)
        self.register(oak_tree());
        self.register(pine_tree());
        self.register(birch_tree());
        self.register(palm_tree());
        self.register(cactus());

        // Trees (Fruit)
        self.register(apple_tree());
        self.register(pear_tree());
        self.register(cherry_tree());
        self.register(orange_tree());
        self.register(banana_tree());
        self.register(olive_tree());

        // Trees (Tropical/Exotic)
        self.register(mahogany_tree());
        self.register(bamboo());
        self.register(mangrove_tree());

        // Trees (Ancient/Special)
        self.register(sequoia_tree());
        self.register(baobab_tree());

        // Fiber plants
        self.register(flax_plant());
        self.register(cotton_plant());
        self.register(hemp_plant());

        // Bushes and shrubs
        self.register(berry_bush());
        self.register(willow_shrub());
        self.register(tea_bush());
        self.register(coffee_bush());

        // Crops (Grains)
        self.register(wheat());
        self.register(barley());
        self.register(corn());
        self.register(rice());

        // Crops (Vegetables)
        self.register(potato());
        self.register(carrot());
        self.register(onion());
        self.register(cabbage());
        self.register(tomato());
        self.register(pumpkin());

        // Flowers and decorative
        self.register(rose());
        self.register(lavender());
        self.register(tulip());
        self.register(sunflower());

        // Grasses and herbs
        self.register(grass());
        self.register(medicinal_herb());
        self.register(mint());
        self.register(sage());

        // Medicinal/Alchemical
        self.register(aloe());
        self.register(ginseng());
        self.register(chamomile());
        self.register(mandrake());

        // Fungi
        self.register(mushroom());
        self.register(poisonous_mushroom());
        self.register(shelf_fungus());

        // Aquatic
        self.register(reeds());
        self.register(lotus());
        self.register(seaweed());
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
        primary_biomes: vec![ClimateZone::Temperate],
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
        primary_biomes: vec![ClimateZone::Arctic, ClimateZone::Temperate],
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
        primary_biomes: vec![ClimateZone::Arctic, ClimateZone::Temperate],
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
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Desert],
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
        primary_biomes: vec![ClimateZone::Desert],
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
        primary_biomes: vec![ClimateZone::Temperate],
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
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
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
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
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
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
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
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
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
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Desert],
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
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("medicinal_herbs".to_string(), 2, 4).at_stage(GrowthStage::Flowering),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn mint() -> PlantSpecies {
    PlantSpecies {
        id: "mint".to_string(),
        name: "Mint".to_string(),
        description: "Refreshing aromatic herb that spreads easily".to_string(),
        health: 8.0,
        growth_time: 80,
        regrows: true,
        regrow_time: 60,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("mint_leaves".to_string(), 3, 6),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn sage() -> PlantSpecies {
    PlantSpecies {
        id: "sage".to_string(),
        name: "Sage".to_string(),
        description: "Woody herb with culinary and medicinal uses".to_string(),
        health: 12.0,
        growth_time: 150,
        regrows: true,
        regrow_time: 100,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        drops: vec![
            PlantDrop::new("sage_leaves".to_string(), 2, 5),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

// ============================================================================
// FRUIT TREES
// ============================================================================

fn apple_tree() -> PlantSpecies {
    PlantSpecies {
        id: "apple_tree".to_string(),
        name: "Apple Tree".to_string(),
        description: "Fruit-bearing tree producing crisp, sweet apples".to_string(),
        health: 160.0,
        growth_time: 4000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 10, 16),
            PlantDrop::new("apples".to_string(), 8, 15).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn pear_tree() -> PlantSpecies {
    PlantSpecies {
        id: "pear_tree".to_string(),
        name: "Pear Tree".to_string(),
        description: "Delicate fruit tree bearing sweet, juicy pears".to_string(),
        health: 155.0,
        growth_time: 3800,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 10, 15),
            PlantDrop::new("pears".to_string(), 6, 12).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn cherry_tree() -> PlantSpecies {
    PlantSpecies {
        id: "cherry_tree".to_string(),
        name: "Cherry Tree".to_string(),
        description: "Beautiful flowering tree with delicious fruit".to_string(),
        health: 145.0,
        growth_time: 3500,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 8, 14),
            PlantDrop::new("cherries".to_string(), 10, 20).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn orange_tree() -> PlantSpecies {
    PlantSpecies {
        id: "orange_tree".to_string(),
        name: "Orange Tree".to_string(),
        description: "Citrus tree with fragrant flowers and vitamin-rich fruit".to_string(),
        health: 150.0,
        growth_time: 3600,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        drops: vec![
            PlantDrop::new("wood".to_string(), 9, 14),
            PlantDrop::new("oranges".to_string(), 8, 16).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn banana_tree() -> PlantSpecies {
    PlantSpecies {
        id: "banana_tree".to_string(),
        name: "Banana Tree".to_string(),
        description: "Fast-growing tropical plant with large edible fruit clusters".to_string(),
        health: 120.0,
        growth_time: 2500,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("plant_fiber".to_string(), 10, 15),
            PlantDrop::new("bananas".to_string(), 5, 10).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false, // Technically not a tree
        size: PlantSize::Medium,
    }
}

fn olive_tree() -> PlantSpecies {
    PlantSpecies {
        id: "olive_tree".to_string(),
        name: "Olive Tree".to_string(),
        description: "Hardy Mediterranean tree producing oil-rich fruit".to_string(),
        health: 180.0,
        growth_time: 4500,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        drops: vec![
            PlantDrop::new("wood".to_string(), 12, 18),
            PlantDrop::new("olives".to_string(), 10, 20).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

// ============================================================================
// TROPICAL/EXOTIC TREES
// ============================================================================

fn mahogany_tree() -> PlantSpecies {
    PlantSpecies {
        id: "mahogany_tree".to_string(),
        name: "Mahogany Tree".to_string(),
        description: "Valuable hardwood tree with rich reddish-brown timber".to_string(),
        health: 250.0,
        growth_time: 6000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("mahogany_wood".to_string(), 20, 30),
            PlantDrop::new("bark".to_string(), 10, 15),
        ],
        is_tree: true,
        size: PlantSize::Huge,
    }
}

fn bamboo() -> PlantSpecies {
    PlantSpecies {
        id: "bamboo".to_string(),
        name: "Bamboo".to_string(),
        description: "Fast-growing woody grass with countless uses".to_string(),
        health: 80.0,
        growth_time: 500,
        regrows: true,
        regrow_time: 300,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        drops: vec![
            PlantDrop::new("bamboo".to_string(), 8, 15),
            PlantDrop::new("bamboo_shoots".to_string(), 2, 4).at_stage(GrowthStage::Growing),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

fn mangrove_tree() -> PlantSpecies {
    PlantSpecies {
        id: "mangrove_tree".to_string(),
        name: "Mangrove Tree".to_string(),
        description: "Coastal tree with salt-tolerant roots that stabilize shorelines".to_string(),
        health: 170.0,
        growth_time: 4200,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 12, 18),
            PlantDrop::new("mangrove_bark".to_string(), 8, 12),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

// ============================================================================
// ANCIENT/SPECIAL TREES
// ============================================================================

fn sequoia_tree() -> PlantSpecies {
    PlantSpecies {
        id: "sequoia_tree".to_string(),
        name: "Sequoia Tree".to_string(),
        description: "Massive ancient conifer, one of the largest living things".to_string(),
        health: 500.0,
        growth_time: 10000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 40, 60),
            PlantDrop::new("bark".to_string(), 20, 30),
            PlantDrop::new("resin".to_string(), 5, 10),
        ],
        is_tree: true,
        size: PlantSize::Huge,
    }
}

fn baobab_tree() -> PlantSpecies {
    PlantSpecies {
        id: "baobab_tree".to_string(),
        name: "Baobab Tree".to_string(),
        description: "Iconic African tree with massive water-storing trunk".to_string(),
        health: 400.0,
        growth_time: 8000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Desert, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 30, 50),
            PlantDrop::new("baobab_fruit".to_string(), 10, 15).at_stage(GrowthStage::Fruiting),
            PlantDrop::new("plant_fiber".to_string(), 15, 25),
        ],
        is_tree: true,
        size: PlantSize::Huge,
    }
}

// ============================================================================
// BUSHES (ADDITIONAL)
// ============================================================================

fn tea_bush() -> PlantSpecies {
    PlantSpecies {
        id: "tea_bush".to_string(),
        name: "Tea Bush".to_string(),
        description: "Evergreen shrub whose leaves produce the world's most popular beverage".to_string(),
        health: 45.0,
        growth_time: 600,
        regrows: true,
        regrow_time: 200,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("tea_leaves".to_string(), 4, 8),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn coffee_bush() -> PlantSpecies {
    PlantSpecies {
        id: "coffee_bush".to_string(),
        name: "Coffee Bush".to_string(),
        description: "Tropical shrub bearing cherries containing stimulating coffee beans".to_string(),
        health: 50.0,
        growth_time: 700,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("coffee_beans".to_string(), 6, 12).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

// ============================================================================
// CROPS - GRAINS
// ============================================================================

fn wheat() -> PlantSpecies {
    PlantSpecies {
        id: "wheat".to_string(),
        name: "Wheat".to_string(),
        description: "Staple grain crop, ground into flour for bread".to_string(),
        health: 15.0,
        growth_time: 250,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wheat".to_string(), 8, 12).at_stage(GrowthStage::Mature),
            PlantDrop::new("straw".to_string(), 4, 6),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn barley() -> PlantSpecies {
    PlantSpecies {
        id: "barley".to_string(),
        name: "Barley".to_string(),
        description: "Hardy grain used for food, brewing, and animal feed".to_string(),
        health: 18.0,
        growth_time: 230,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        drops: vec![
            PlantDrop::new("barley".to_string(), 8, 14).at_stage(GrowthStage::Mature),
            PlantDrop::new("straw".to_string(), 3, 5),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn corn() -> PlantSpecies {
    PlantSpecies {
        id: "corn".to_string(),
        name: "Corn".to_string(),
        description: "Tall grain plant producing large edible kernels on cobs".to_string(),
        health: 25.0,
        growth_time: 280,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("corn".to_string(), 4, 8).at_stage(GrowthStage::Mature),
            PlantDrop::new("corn_stalks".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

fn rice() -> PlantSpecies {
    PlantSpecies {
        id: "rice".to_string(),
        name: "Rice".to_string(),
        description: "Water-loving grain, staple food for billions".to_string(),
        health: 12.0,
        growth_time: 300,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        drops: vec![
            PlantDrop::new("rice".to_string(), 10, 16).at_stage(GrowthStage::Mature),
            PlantDrop::new("rice_straw".to_string(), 3, 5),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

// ============================================================================
// CROPS - VEGETABLES
// ============================================================================

fn potato() -> PlantSpecies {
    PlantSpecies {
        id: "potato".to_string(),
        name: "Potato".to_string(),
        description: "Underground tuber, highly nutritious and storable".to_string(),
        health: 20.0,
        growth_time: 200,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        drops: vec![
            PlantDrop::new("potatoes".to_string(), 4, 8).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn carrot() -> PlantSpecies {
    PlantSpecies {
        id: "carrot".to_string(),
        name: "Carrot".to_string(),
        description: "Root vegetable rich in vitamins, stores well".to_string(),
        health: 12.0,
        growth_time: 150,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("carrots".to_string(), 3, 6).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn onion() -> PlantSpecies {
    PlantSpecies {
        id: "onion".to_string(),
        name: "Onion".to_string(),
        description: "Pungent bulb vegetable, essential flavoring and preservative".to_string(),
        health: 14.0,
        growth_time: 180,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        drops: vec![
            PlantDrop::new("onions".to_string(), 2, 5).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn cabbage() -> PlantSpecies {
    PlantSpecies {
        id: "cabbage".to_string(),
        name: "Cabbage".to_string(),
        description: "Leafy vegetable forming dense heads, stores well".to_string(),
        health: 22.0,
        growth_time: 220,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        drops: vec![
            PlantDrop::new("cabbage".to_string(), 1, 2).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn tomato() -> PlantSpecies {
    PlantSpecies {
        id: "tomato".to_string(),
        name: "Tomato".to_string(),
        description: "Juicy fruit (botanically) used as a vegetable, needs warmth".to_string(),
        health: 16.0,
        growth_time: 240,
        regrows: true,
        regrow_time: 80,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("tomatoes".to_string(), 3, 6).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn pumpkin() -> PlantSpecies {
    PlantSpecies {
        id: "pumpkin".to_string(),
        name: "Pumpkin".to_string(),
        description: "Large vine fruit, nutritious flesh and edible seeds".to_string(),
        health: 30.0,
        growth_time: 280,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("pumpkin".to_string(), 1, 2).at_stage(GrowthStage::Mature),
            PlantDrop::new("pumpkin_seeds".to_string(), 10, 20).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

// ============================================================================
// FLOWERS AND DECORATIVE
// ============================================================================

fn rose() -> PlantSpecies {
    PlantSpecies {
        id: "rose".to_string(),
        name: "Rose".to_string(),
        description: "Beautiful flowering plant with thorny stems and fragrant petals".to_string(),
        health: 18.0,
        growth_time: 400,
        regrows: true,
        regrow_time: 150,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("rose_petals".to_string(), 4, 8).at_stage(GrowthStage::Flowering),
            PlantDrop::new("rose_hips".to_string(), 2, 4).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn lavender() -> PlantSpecies {
    PlantSpecies {
        id: "lavender".to_string(),
        name: "Lavender".to_string(),
        description: "Aromatic herb with calming purple flowers".to_string(),
        health: 14.0,
        growth_time: 350,
        regrows: true,
        regrow_time: 120,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        drops: vec![
            PlantDrop::new("lavender".to_string(), 3, 6).at_stage(GrowthStage::Flowering),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn tulip() -> PlantSpecies {
    PlantSpecies {
        id: "tulip".to_string(),
        name: "Tulip".to_string(),
        description: "Elegant spring flower growing from bulbs".to_string(),
        health: 8.0,
        growth_time: 200,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("tulip_petals".to_string(), 2, 4).at_stage(GrowthStage::Flowering),
            PlantDrop::new("tulip_bulb".to_string(), 1, 1),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn sunflower() -> PlantSpecies {
    PlantSpecies {
        id: "sunflower".to_string(),
        name: "Sunflower".to_string(),
        description: "Tall plant with large yellow flowers that track the sun".to_string(),
        health: 28.0,
        growth_time: 250,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("sunflower_petals".to_string(), 8, 15).at_stage(GrowthStage::Flowering),
            PlantDrop::new("sunflower_seeds".to_string(), 20, 40).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

// ============================================================================
// MEDICINAL/ALCHEMICAL
// ============================================================================

fn aloe() -> PlantSpecies {
    PlantSpecies {
        id: "aloe".to_string(),
        name: "Aloe Vera".to_string(),
        description: "Succulent with gel-filled leaves, powerful healing properties".to_string(),
        health: 20.0,
        growth_time: 300,
        regrows: true,
        regrow_time: 150,
        primary_biomes: vec![ClimateZone::Desert, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("aloe_gel".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn ginseng() -> PlantSpecies {
    PlantSpecies {
        id: "ginseng".to_string(),
        name: "Ginseng".to_string(),
        description: "Rare root with potent medicinal and energizing properties".to_string(),
        health: 15.0,
        growth_time: 800,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("ginseng_root".to_string(), 1, 2).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn chamomile() -> PlantSpecies {
    PlantSpecies {
        id: "chamomile".to_string(),
        name: "Chamomile".to_string(),
        description: "Daisy-like flower with calming and medicinal properties".to_string(),
        health: 10.0,
        growth_time: 180,
        regrows: true,
        regrow_time: 90,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("chamomile_flowers".to_string(), 3, 6).at_stage(GrowthStage::Flowering),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn mandrake() -> PlantSpecies {
    PlantSpecies {
        id: "mandrake".to_string(),
        name: "Mandrake".to_string(),
        description: "Mystical root plant, powerful but dangerous in alchemy".to_string(),
        health: 18.0,
        growth_time: 600,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("mandrake_root".to_string(), 1, 1).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

// ============================================================================
// FUNGI
// ============================================================================

fn mushroom() -> PlantSpecies {
    PlantSpecies {
        id: "mushroom".to_string(),
        name: "Mushroom".to_string(),
        description: "Common edible fungus found in forests".to_string(),
        health: 3.0,
        growth_time: 60,
        regrows: true,
        regrow_time: 40,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("mushrooms".to_string(), 1, 3),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn poisonous_mushroom() -> PlantSpecies {
    PlantSpecies {
        id: "poisonous_mushroom".to_string(),
        name: "Poisonous Mushroom".to_string(),
        description: "Toxic fungus with bright warning colors, dangerous but useful in alchemy".to_string(),
        health: 3.0,
        growth_time: 70,
        regrows: true,
        regrow_time: 50,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("poison_mushrooms".to_string(), 1, 2),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn shelf_fungus() -> PlantSpecies {
    PlantSpecies {
        id: "shelf_fungus".to_string(),
        name: "Shelf Fungus".to_string(),
        description: "Hardy bracket fungus growing on trees, useful for tinder and medicine".to_string(),
        health: 8.0,
        growth_time: 200,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic, ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("tinder_fungus".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

// ============================================================================
// AQUATIC PLANTS
// ============================================================================

fn reeds() -> PlantSpecies {
    PlantSpecies {
        id: "reeds".to_string(),
        name: "Reeds".to_string(),
        description: "Tall wetland grasses useful for weaving and construction".to_string(),
        health: 12.0,
        growth_time: 150,
        regrows: true,
        regrow_time: 100,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("reeds".to_string(), 4, 8),
            PlantDrop::new("plant_fiber".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

fn lotus() -> PlantSpecies {
    PlantSpecies {
        id: "lotus".to_string(),
        name: "Lotus".to_string(),
        description: "Sacred water plant with beautiful flowers and edible roots".to_string(),
        health: 15.0,
        growth_time: 250,
        regrows: true,
        regrow_time: 120,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        drops: vec![
            PlantDrop::new("lotus_petals".to_string(), 3, 6).at_stage(GrowthStage::Flowering),
            PlantDrop::new("lotus_root".to_string(), 1, 3).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn seaweed() -> PlantSpecies {
    PlantSpecies {
        id: "seaweed".to_string(),
        name: "Seaweed".to_string(),
        description: "Marine algae, nutritious and useful for fertilizer".to_string(),
        health: 8.0,
        growth_time: 100,
        regrows: true,
        regrow_time: 60,
        primary_biomes: vec![ClimateZone::Tropical, ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        drops: vec![
            PlantDrop::new("seaweed".to_string(), 3, 6),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

// ============================================================================
// PLANT INSTANCES AND MANAGEMENT
// ============================================================================

/// Individual plant instance in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plant {
    pub id: Uuid,
    pub species_id: String,
    pub position: (i32, i32),
    pub current_health: f32,
    pub max_health: f32,
    pub growth_stage: GrowthStage,
    pub growth_progress: f32, // 0.0 to 1.0 for current stage
    pub age_ticks: u32,
    pub is_harvestable: bool,
    pub has_been_harvested: bool,
    pub regrow_timer: u32,
    pub planted_by: Option<Uuid>, // Agent who planted it (for farming)
    pub is_cultivated: bool, // Whether it's a farm plant vs wild
}

impl Plant {
    /// Create a new plant instance
    pub fn new(species_id: String, position: (i32, i32)) -> Self {
        Self {
            id: Uuid::new_v4(),
            species_id,
            position,
            current_health: 0.0, // Set from species
            max_health: 0.0,
            growth_stage: GrowthStage::Seedling,
            growth_progress: 0.0,
            age_ticks: 0,
            is_harvestable: false,
            has_been_harvested: false,
            regrow_timer: 0,
            planted_by: None,
            is_cultivated: false,
        }
    }

    /// Initialize with species data
    pub fn with_species(mut self, species: &PlantSpecies) -> Self {
        self.max_health = species.health;
        self.current_health = species.health;
        self
    }

    /// Mark as cultivated/farmed
    pub fn cultivated(mut self, planter_id: Uuid) -> Self {
        self.is_cultivated = true;
        self.planted_by = Some(planter_id);
        self
    }

    /// Advance growth by one tick
    pub fn grow(&mut self, species: &PlantSpecies) -> bool {
        if self.has_been_harvested && self.regrow_timer > 0 {
            self.regrow_timer -= 1;
            if self.regrow_timer == 0 {
                // Reset for regrowth
                self.growth_stage = GrowthStage::Seedling;
                self.growth_progress = 0.0;
                self.has_been_harvested = false;
                self.is_harvestable = false;
            }
            return false;
        }

        if self.has_been_harvested && !species.regrows {
            return false; // Dead plant
        }

        self.age_ticks += 1;

        // Calculate growth for current stage
        let stage_duration = self.stage_duration(species);
        self.growth_progress += 1.0 / stage_duration as f32;

        if self.growth_progress >= 1.0 {
            // Advance to next stage
            self.growth_progress = 0.0;
            match self.growth_stage {
                GrowthStage::Seedling => self.growth_stage = GrowthStage::Growing,
                GrowthStage::Growing => self.growth_stage = GrowthStage::Mature,
                GrowthStage::Mature => self.growth_stage = GrowthStage::Flowering,
                GrowthStage::Flowering => self.growth_stage = GrowthStage::Fruiting,
                GrowthStage::Fruiting => {
                    self.is_harvestable = true;
                    return true; // Fully grown
                }
            }
        }

        // Check if harvestable at current stage
        if self.growth_stage == GrowthStage::Mature || self.growth_stage == GrowthStage::Fruiting {
            self.is_harvestable = true;
        }

        false
    }

    fn stage_duration(&self, species: &PlantSpecies) -> u32 {
        // Divide total growth time among stages
        species.growth_time / 5
    }

    /// Harvest the plant
    pub fn harvest(&mut self, species: &PlantSpecies) -> Vec<PlantDrop> {
        if !self.is_harvestable {
            return Vec::new();
        }

        self.has_been_harvested = true;

        if species.regrows {
            self.regrow_timer = species.regrow_time;
        }

        // Return drops that are available at current stage
        species.drops
            .iter()
            .filter(|drop| {
                drop.required_stage.is_none() ||
                drop.required_stage == Some(self.growth_stage)
            })
            .cloned()
            .collect()
    }

    /// Damage the plant
    pub fn damage(&mut self, amount: f32) -> bool {
        self.current_health -= amount;
        self.current_health <= 0.0
    }

    /// Get plant status string
    pub fn status(&self) -> String {
        if self.has_been_harvested {
            if self.regrow_timer > 0 {
                format!("Harvested (regrows in {} ticks)", self.regrow_timer)
            } else {
                "Dead".to_string()
            }
        } else {
            format!("{:?} ({:.0}% through stage)", self.growth_stage, self.growth_progress * 100.0)
        }
    }
}

/// Manages plant population and growth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantManager {
    plants: Vec<Plant>,
    max_population: usize,
    natural_spawn_rate: f32,
    #[serde(skip)]
    registry: Option<FloraRegistry>,
}

impl PlantManager {
    pub fn new(max_population: usize) -> Self {
        Self {
            plants: Vec::new(),
            max_population,
            natural_spawn_rate: 0.01,
            registry: Some(FloraRegistry::new()),
        }
    }

    pub fn with_registry(mut self, registry: FloraRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Spawn a plant at a position
    pub fn spawn_plant(&mut self, species_id: String, position: (i32, i32)) -> Option<Uuid> {
        if self.plants.len() >= self.max_population {
            return None;
        }

        let species = self.registry.as_ref()?.get(&species_id)?;

        let plant = Plant::new(species_id.clone(), position)
            .with_species(species);

        let id = plant.id;
        self.plants.push(plant);
        Some(id)
    }

    /// Spawn a cultivated plant (farmed)
    pub fn plant_crop(&mut self, species_id: String, position: (i32, i32), planter_id: Uuid) -> Option<Uuid> {
        if self.plants.len() >= self.max_population {
            return None;
        }

        let species = self.registry.as_ref()?.get(&species_id)?;

        let plant = Plant::new(species_id.clone(), position)
            .with_species(species)
            .cultivated(planter_id);

        let id = plant.id;
        self.plants.push(plant);
        Some(id)
    }

    /// Spawn multiple plants in an area (forest, field, etc.)
    pub fn spawn_patch(&mut self, species_id: String, center: (i32, i32), radius: u32, density: f32) -> Vec<Uuid> {
        let mut spawned = Vec::new();
        let count = ((radius * radius) as f32 * density) as u32;

        for _ in 0..count {
            let offset_x = (rand::random::<i32>() % (radius as i32 * 2)) - radius as i32;
            let offset_y = (rand::random::<i32>() % (radius as i32 * 2)) - radius as i32;

            let pos = (center.0 + offset_x, center.1 + offset_y);

            if let Some(id) = self.spawn_plant(species_id.clone(), pos) {
                spawned.push(id);
            }
        }

        spawned
    }

    /// Get plant by ID
    pub fn get(&self, id: &Uuid) -> Option<&Plant> {
        self.plants.iter().find(|p| &p.id == id)
    }

    /// Get mutable plant by ID
    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut Plant> {
        self.plants.iter_mut().find(|p| &p.id == id)
    }

    /// Get all plants at a position
    pub fn get_at_position(&self, position: (i32, i32)) -> Vec<&Plant> {
        self.plants.iter().filter(|p| p.position == position).collect()
    }

    /// Get all plants in radius
    pub fn get_in_radius(&self, center: (i32, i32), radius: f32) -> Vec<&Plant> {
        self.plants
            .iter()
            .filter(|p| {
                let dx = (p.position.0 - center.0) as f32;
                let dy = (p.position.1 - center.1) as f32;
                (dx * dx + dy * dy).sqrt() <= radius
            })
            .collect()
    }

    /// Get harvestable plants in radius
    pub fn get_harvestable_in_radius(&self, center: (i32, i32), radius: f32) -> Vec<&Plant> {
        self.get_in_radius(center, radius)
            .into_iter()
            .filter(|p| p.is_harvestable && !p.has_been_harvested)
            .collect()
    }

    /// Harvest a plant
    pub fn harvest_plant(&mut self, id: &Uuid) -> Option<Vec<PlantDrop>> {
        let plant = self.get_mut(id)?;
        let species = self.registry.as_ref()?.get(&plant.species_id)?;
        Some(plant.harvest(species))
    }

    /// Get all plants
    pub fn get_all(&self) -> &Vec<Plant> {
        &self.plants
    }

    /// Get species from registry
    pub fn get_species(&self, species_id: &str) -> Option<&PlantSpecies> {
        self.registry.as_ref()?.get(species_id)
    }

    /// Tick all plants (growth, regrowth)
    pub fn tick(&mut self) {
        let registry = match &self.registry {
            Some(r) => r,
            None => return,
        };

        // Remove dead plants that don't regrow
        self.plants.retain(|p| {
            if p.has_been_harvested {
                if let Some(species) = registry.get(&p.species_id) {
                    species.regrows || p.regrow_timer > 0
                } else {
                    false
                }
            } else {
                true
            }
        });

        // Grow plants
        for plant in &mut self.plants {
            if let Some(species) = registry.get(&plant.species_id) {
                plant.grow(species);
            }
        }
    }

    /// Get count of plants by species
    pub fn count_species(&self, species_id: &str) -> usize {
        self.plants.iter().filter(|p| p.species_id == species_id).count()
    }

    /// Get count of harvestable plants by species
    pub fn count_harvestable(&self, species_id: &str) -> usize {
        self.plants
            .iter()
            .filter(|p| p.species_id == species_id && p.is_harvestable && !p.has_been_harvested)
            .count()
    }

    /// Get all plants
    pub fn all_plants(&self) -> &Vec<Plant> {
        &self.plants
    }

    /// Get total plant count
    pub fn total_count(&self) -> usize {
        self.plants.len()
    }

    /// Get summary of all plants
    pub fn plant_summary(&self) -> Vec<String> {
        let mut summaries = Vec::new();

        for plant in &self.plants {
            if let Some(registry) = &self.registry {
                if let Some(species) = registry.get(&plant.species_id) {
                    let cultivated = if plant.is_cultivated { " (cultivated)" } else { "" };
                    summaries.push(format!(
                        "{}{} at ({}, {}): {}",
                        species.name,
                        cultivated,
                        plant.position.0,
                        plant.position.1,
                        plant.status()
                    ));
                }
            }
        }

        summaries
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

        let temperate_plants = registry.get_by_biome(ClimateZone::Temperate);
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
