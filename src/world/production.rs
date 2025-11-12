// src/world/production.rs
//! Production and crafting system for agents with professions.

use serde::{Deserialize, Serialize};
use crate::agents::profession::JobType;
use crate::world::{ResourceType, ItemType};

/// Quality tier of a crafted item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quality {
    Poor,       // 0-20 skill
    Common,     // 21-40 skill
    Good,       // 41-60 skill
    Excellent,  // 61-80 skill
    Masterwork, // 81-100 skill
}

impl Quality {
    /// Get quality from skill level
    pub fn from_skill(skill: u8) -> Self {
        match skill {
            0..=20 => Quality::Poor,
            21..=40 => Quality::Common,
            41..=60 => Quality::Good,
            61..=80 => Quality::Excellent,
            _ => Quality::Masterwork,
        }
    }

    /// Get output quantity multiplier
    pub fn output_multiplier(&self) -> f32 {
        match self {
            Quality::Poor => 0.8,
            Quality::Common => 1.0,
            Quality::Good => 1.2,
            Quality::Excellent => 1.4,
            Quality::Masterwork => 1.6,
        }
    }

    /// Get production time multiplier (lower is faster)
    pub fn time_multiplier(&self) -> f32 {
        match self {
            Quality::Poor => 1.2,       // 20% slower
            Quality::Common => 1.0,     // Normal speed
            Quality::Good => 0.85,      // 15% faster
            Quality::Excellent => 0.7,  // 30% faster
            Quality::Masterwork => 0.5, // 50% faster
        }
    }
}

/// A resource requirement for a recipe
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRequirement {
    pub resource_type: ResourceType,
    pub amount: u32,
}

impl ResourceRequirement {
    pub fn new(resource_type: ResourceType, amount: u32) -> Self {
        Self { resource_type, amount }
    }
}

/// A production output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductionOutput {
    pub item_type: ItemType,
    pub base_amount: u32,
}

impl ProductionOutput {
    pub fn new(item_type: ItemType, base_amount: u32) -> Self {
        Self { item_type, base_amount }
    }
}

/// A crafting recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub name: &'static str,
    pub job: JobType,
    pub inputs: Vec<ResourceRequirement>,
    pub outputs: Vec<ProductionOutput>,
    pub base_time: u32, // Base time in ticks
}

impl Recipe {
    /// Calculate actual output based on quality
    pub fn calculate_output(&self, quality: Quality) -> Vec<(ItemType, u32)> {
        self.outputs
            .iter()
            .map(|output| {
                let amount = (output.base_amount as f32 * quality.output_multiplier()).ceil() as u32;
                (output.item_type, amount.max(1))
            })
            .collect()
    }

    /// Calculate production time based on quality
    pub fn calculate_time(&self, quality: Quality) -> u32 {
        (self.base_time as f32 * quality.time_multiplier()).ceil() as u32
    }
}

/// Get all recipes for a specific job
pub fn get_job_recipes(job: JobType) -> Vec<Recipe> {
    match job {
        // Food Processing
        JobType::Miller => vec![
            Recipe {
                name: "Grind Grain into Flour",
                job: JobType::Miller,
                inputs: vec![ResourceRequirement::new(ResourceType::Grain, 2)],
                outputs: vec![ProductionOutput::new(ItemType::Flour, 2)],
                base_time: 50,
            },
        ],

        JobType::Butcher => vec![
            Recipe {
                name: "Process Meat",
                job: JobType::Butcher,
                inputs: vec![ResourceRequirement::new(ResourceType::Meat, 1)],
                outputs: vec![ProductionOutput::new(ItemType::Meat, 2)],
                base_time: 40,
            },
        ],

        JobType::Baker => vec![
            Recipe {
                name: "Bake Bread",
                job: JobType::Baker,
                inputs: vec![ResourceRequirement::new(ResourceType::Flour, 1)],
                outputs: vec![ProductionOutput::new(ItemType::Bread, 2)],
                base_time: 60,
            },
        ],

        JobType::Brewer => vec![
            Recipe {
                name: "Brew Ale",
                job: JobType::Brewer,
                inputs: vec![ResourceRequirement::new(ResourceType::Grain, 3)],
                outputs: vec![ProductionOutput::new(ItemType::Ale, 2)],
                base_time: 100,
            },
        ],

        JobType::Cheesemaker => vec![
            Recipe {
                name: "Make Cheese",
                job: JobType::Cheesemaker,
                inputs: vec![ResourceRequirement::new(ResourceType::Milk, 3)],
                outputs: vec![ProductionOutput::new(ItemType::Cheese, 1)],
                base_time: 80,
            },
        ],

        // Material Processing
        JobType::Tanner => vec![
            Recipe {
                name: "Tan Hides into Leather",
                job: JobType::Tanner,
                inputs: vec![ResourceRequirement::new(ResourceType::Hides, 2)],
                outputs: vec![ProductionOutput::new(ItemType::Leather, 2)],
                base_time: 70,
            },
        ],

        JobType::Potter => vec![
            Recipe {
                name: "Fire Pottery",
                job: JobType::Potter,
                inputs: vec![ResourceRequirement::new(ResourceType::Clay, 1)],
                outputs: vec![ProductionOutput::new(ItemType::Pottery, 1)],
                base_time: 80,
            },
        ],

        JobType::Weaver => vec![
            Recipe {
                name: "Weave Cloth",
                job: JobType::Weaver,
                inputs: vec![ResourceRequirement::new(ResourceType::Flax, 2)],
                outputs: vec![ProductionOutput::new(ItemType::Cloth, 1)],
                base_time: 60,
            },
            Recipe {
                name: "Weave Linen",
                job: JobType::Weaver,
                inputs: vec![ResourceRequirement::new(ResourceType::Cotton, 2)],
                outputs: vec![ProductionOutput::new(ItemType::Linen, 1)],
                base_time: 70,
            },
        ],

        JobType::Spinner => vec![
            Recipe {
                name: "Spin Wool into Cloth",
                job: JobType::Spinner,
                inputs: vec![ResourceRequirement::new(ResourceType::Wool, 2)],
                outputs: vec![ProductionOutput::new(ItemType::Cloth, 1)],
                base_time: 50,
            },
        ],

        JobType::Glassblower => vec![
            Recipe {
                name: "Blow Glass",
                job: JobType::Glassblower,
                inputs: vec![ResourceRequirement::new(ResourceType::Sand, 2)],
                outputs: vec![ProductionOutput::new(ItemType::Glass, 1)],
                base_time: 90,
            },
        ],

        JobType::Dyer => vec![
            Recipe {
                name: "Create Dye",
                job: JobType::Dyer,
                inputs: vec![ResourceRequirement::new(ResourceType::Herbs, 3)],
                outputs: vec![ProductionOutput::new(ItemType::Dye, 2)],
                base_time: 40,
            },
        ],

        JobType::Ropemaker => vec![
            Recipe {
                name: "Make Rope",
                job: JobType::Ropemaker,
                inputs: vec![ResourceRequirement::new(ResourceType::Flax, 2)],
                outputs: vec![ProductionOutput::new(ItemType::Rope, 3)],
                base_time: 50,
            },
        ],

        JobType::Brickmaker => vec![
            Recipe {
                name: "Fire Bricks",
                job: JobType::Brickmaker,
                inputs: vec![ResourceRequirement::new(ResourceType::Clay, 3)],
                outputs: vec![ProductionOutput::new(ItemType::Bricks, 4)],
                base_time: 70,
            },
        ],

        JobType::CharcoalMaker => vec![
            Recipe {
                name: "Make Charcoal",
                job: JobType::CharcoalMaker,
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 4)],
                outputs: vec![ProductionOutput::new(ItemType::Charcoal, 3)],
                base_time: 100,
            },
        ],

        JobType::Papermaker => vec![
            Recipe {
                name: "Make Paper",
                job: JobType::Papermaker,
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 2)],
                outputs: vec![ProductionOutput::new(ItemType::Paper, 3)],
                base_time: 60,
            },
        ],

        // Crafting - Tools and Weapons
        JobType::Carpenter => vec![
            Recipe {
                name: "Craft Wooden Axe",
                job: JobType::Carpenter,
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenAxe, 1)],
                base_time: 80,
            },
            Recipe {
                name: "Craft Wooden Pickaxe",
                job: JobType::Carpenter,
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenPickaxe, 1)],
                base_time: 80,
            },
            Recipe {
                name: "Craft Wooden Hammer",
                job: JobType::Carpenter,
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenHammer, 1)],
                base_time: 80,
            },
            Recipe {
                name: "Craft Furniture",
                job: JobType::Carpenter,
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 5)],
                outputs: vec![ProductionOutput::new(ItemType::Furniture, 1)],
                base_time: 120,
            },
        ],

        JobType::Stonemason => vec![
            Recipe {
                name: "Craft Stone Axe",
                job: JobType::Stonemason,
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Stone, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::StoneAxe, 1)],
                base_time: 90,
            },
            Recipe {
                name: "Craft Stone Pickaxe",
                job: JobType::Stonemason,
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Stone, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::StonePickaxe, 1)],
                base_time: 90,
            },
            Recipe {
                name: "Craft Stone Hammer",
                job: JobType::Stonemason,
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Stone, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::StoneHammer, 1)],
                base_time: 90,
            },
        ],

        JobType::Blacksmith => vec![
            Recipe {
                name: "Forge Iron Axe",
                job: JobType::Blacksmith,
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronAxe, 1)],
                base_time: 100,
            },
            Recipe {
                name: "Forge Iron Pickaxe",
                job: JobType::Blacksmith,
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronPickaxe, 1)],
                base_time: 100,
            },
            Recipe {
                name: "Forge Iron Hammer",
                job: JobType::Blacksmith,
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronHammer, 1)],
                base_time: 100,
            },
            Recipe {
                name: "Forge Iron Sword",
                job: JobType::Blacksmith,
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 3),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronSword, 1)],
                base_time: 120,
            },
        ],

        JobType::Armorer => vec![
            Recipe {
                name: "Craft Leather Armor",
                job: JobType::Armorer,
                inputs: vec![ResourceRequirement::new(ResourceType::Leather, 4)],
                outputs: vec![ProductionOutput::new(ItemType::LeatherArmor, 1)],
                base_time: 100,
            },
            Recipe {
                name: "Forge Iron Armor",
                job: JobType::Armorer,
                inputs: vec![ResourceRequirement::new(ResourceType::Iron, 6)],
                outputs: vec![ProductionOutput::new(ItemType::IronArmor, 1)],
                base_time: 150,
            },
            Recipe {
                name: "Forge Steel Armor",
                job: JobType::Armorer,
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 8),
                    ResourceRequirement::new(ResourceType::Charcoal, 2),
                ],
                outputs: vec![ProductionOutput::new(ItemType::SteelArmor, 1)],
                base_time: 200,
            },
        ],

        JobType::Goldsmith => vec![
            Recipe {
                name: "Craft Jewelry",
                job: JobType::Goldsmith,
                inputs: vec![ResourceRequirement::new(ResourceType::Iron, 1)],
                outputs: vec![ProductionOutput::new(ItemType::Jewelry, 1)],
                base_time: 120,
            },
        ],

        JobType::Bowyer => vec![
            Recipe {
                name: "Craft Wooden Bow",
                job: JobType::Bowyer,
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenBow, 1)],
                base_time: 90,
            },
            Recipe {
                name: "Craft Iron Bow",
                job: JobType::Bowyer,
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Wood, 2),
                    ResourceRequirement::new(ResourceType::Iron, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronBow, 1)],
                base_time: 110,
            },
        ],

        JobType::Fletcher => vec![
            Recipe {
                name: "Craft Wooden Spear",
                job: JobType::Fletcher,
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 2)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenSpear, 1)],
                base_time: 50,
            },
            Recipe {
                name: "Craft Stone Spear",
                job: JobType::Fletcher,
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Wood, 1),
                    ResourceRequirement::new(ResourceType::Stone, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::StoneSpear, 1)],
                base_time: 60,
            },
        ],

        // Textile & Leather Goods
        JobType::Tailor => vec![
            Recipe {
                name: "Sew Clothing",
                job: JobType::Tailor,
                inputs: vec![ResourceRequirement::new(ResourceType::Cloth, 3)],
                outputs: vec![ProductionOutput::new(ItemType::Clothing, 1)],
                base_time: 80,
            },
        ],

        JobType::Cobbler => vec![
            Recipe {
                name: "Make Shoes",
                job: JobType::Cobbler,
                inputs: vec![ResourceRequirement::new(ResourceType::Leather, 2)],
                outputs: vec![ProductionOutput::new(ItemType::Shoes, 1)],
                base_time: 70,
            },
        ],

        JobType::Leatherworker => vec![
            Recipe {
                name: "Craft Leather Goods",
                job: JobType::Leatherworker,
                inputs: vec![ResourceRequirement::new(ResourceType::Leather, 2)],
                outputs: vec![ProductionOutput::new(ItemType::LeatherArmor, 1)],
                base_time: 90,
            },
        ],

        // Jobs without recipes (gathering, services, etc.)
        _ => vec![],
    }
}

/// Get primary recipe for a job (most commonly used)
pub fn get_primary_recipe(job: JobType) -> Option<Recipe> {
    get_job_recipes(job).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_from_skill() {
        assert_eq!(Quality::from_skill(10), Quality::Poor);
        assert_eq!(Quality::from_skill(30), Quality::Common);
        assert_eq!(Quality::from_skill(50), Quality::Good);
        assert_eq!(Quality::from_skill(70), Quality::Excellent);
        assert_eq!(Quality::from_skill(90), Quality::Masterwork);
    }

    #[test]
    fn test_quality_multipliers() {
        let poor = Quality::Poor;
        assert_eq!(poor.output_multiplier(), 0.8);
        assert!(poor.time_multiplier() > 1.0);

        let master = Quality::Masterwork;
        assert_eq!(master.output_multiplier(), 1.6);
        assert!(master.time_multiplier() < 1.0);
    }

    #[test]
    fn test_recipe_calculation() {
        let recipe = Recipe {
            name: "Test Recipe",
            job: JobType::Baker,
            inputs: vec![ResourceRequirement::new(ResourceType::Flour, 1)],
            outputs: vec![ProductionOutput::new(ItemType::Bread, 2)],
            base_time: 100,
        };

        // Poor quality produces less
        let poor_output = recipe.calculate_output(Quality::Poor);
        assert_eq!(poor_output[0].1, 2); // 2 * 0.8 = 1.6 -> 2 (ceil)

        // Masterwork produces more
        let master_output = recipe.calculate_output(Quality::Masterwork);
        assert_eq!(master_output[0].1, 4); // 2 * 1.6 = 3.2 -> 4 (ceil)

        // Time calculation
        let poor_time = recipe.calculate_time(Quality::Poor);
        assert!(poor_time > recipe.base_time);

        let master_time = recipe.calculate_time(Quality::Masterwork);
        assert!(master_time < recipe.base_time);
    }

    #[test]
    fn test_job_recipes() {
        // Miller should have recipes
        let miller_recipes = get_job_recipes(JobType::Miller);
        assert!(!miller_recipes.is_empty());
        assert_eq!(miller_recipes[0].job, JobType::Miller);

        // Carpenter should have multiple recipes
        let carpenter_recipes = get_job_recipes(JobType::Carpenter);
        assert!(carpenter_recipes.len() >= 3);

        // Unemployed should have no recipes
        let unemployed_recipes = get_job_recipes(JobType::Unemployed);
        assert!(unemployed_recipes.is_empty());
    }

    #[test]
    fn test_primary_recipe() {
        assert!(get_primary_recipe(JobType::Baker).is_some());
        assert!(get_primary_recipe(JobType::Unemployed).is_none());
    }
}
