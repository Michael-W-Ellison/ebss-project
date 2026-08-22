// src/world/resources.rs
//! Resource nodes and harvestable materials.

use serde::{Deserialize, Serialize};
use crate::world::Position;

/// Types of resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    // === Basic Resources (Existing) ===
    Wood,
    Stone,
    Iron,
    Food, // Generic food (berries, generic edibles)
    Water, // Fresh water from rivers, wells, springs

    // === Raw Materials (Agricultural) ===
    Grain,      // Wheat, barley, etc. - for flour, bread, beer
    Flax,       // For linen, rope
    Herbs,      // For medicine, dyes
    Cotton,     // For cloth

    // === Raw Materials (Animal) ===
    Hides,      // Raw animal skins - for leather
    Wool,       // From sheep - for cloth
    Meat,       // Butchered meat
    Milk,       // For cheese, butter
    Fish,       // From fishing
    Honey,      // From beekeeping

    // === Raw Materials (Mineral) ===
    Clay,       // For bricks, pottery
    Sand,       // For glass
    Coal,       // For fuel/charcoal

    // === Processed Materials ===
    Flour,      // Grain → Miller → Flour
    Leather,    // Hides → Tanner → Leather
    Cloth,      // Flax/Wool/Cotton → Weaver → Cloth
    Linen,      // Flax → Weaver → Linen (specific cloth)
    Glass,      // Sand → Glassblower → Glass
    Bricks,     // Clay → Brickmaker → Bricks
    Charcoal,   // Wood → Charcoal Maker → Charcoal
    Rope,       // Flax → Ropemaker → Rope
    Paper,      // Various → Papermaker → Paper
    Dye,        // Herbs → Dyer → Dye

    // === Finished Goods (Food) ===
    Bread,      // Flour → Baker → Bread
    Ale,        // Grain → Brewer → Ale
    Cheese,     // Milk → Cheesemaker → Cheese

    // === Finished Goods (Items) ===
    Clothing,   // Cloth → Tailor → Clothing
    Shoes,      // Leather → Cobbler → Shoes
    Tools,      // Wood + Iron → Carpenter/Blacksmith → Tools
    Weapons,    // Wood + Iron → Bowyer/Blacksmith → Weapons
    Armor,      // Leather/Iron → Leatherworker/Armorer → Armor
    Pottery,    // Clay → Potter → Pottery
    Furniture,  // Wood → Carpenter → Furniture
    Jewelry,    // Iron/Gold → Goldsmith → Jewelry
}

impl ResourceType {
    /// How strongly this gives itself away by smell where it lies untouched,
    /// as a fraction of an agent's full smelling range.
    ///
    /// Human noses are poor. Berries on the bush and standing grain are close
    /// to odourless until you are almost on top of them - they are found by
    /// looking, not by sniffing. Flesh carries further. Nothing raw on the
    /// ground competes with cooking or with rot, which are what a nose is
    /// actually good for.
    pub fn raw_scent_strength(&self) -> f32 {
        match self {
            // Barely detectable: you have to be standing among them
            ResourceType::Food | ResourceType::Grain | ResourceType::Herbs => 0.08,

            // Flesh gives itself away from further off
            ResourceType::Meat | ResourceType::Fish => 0.24,

            // Damp ground and vegetation, faintly
            ResourceType::Water => 0.12,

            // Wood, stone and ore have no smell worth the name
            _ => 0.0,
        }
    }

    /// Whether an agent can eat this straight from the land.
    ///
    /// The single answer to "is this food", used by foraging, by what an agent
    /// remembers seeing, and by the scents the world gives off.
    pub fn is_edible(&self) -> bool {
        matches!(
            self,
            ResourceType::Food | ResourceType::Grain | ResourceType::Fish | ResourceType::Meat
        )
    }

    /// Get ASCII character for rendering
    pub fn ascii_char(&self) -> char {
        match self {
            // Basic
            ResourceType::Wood => 't',
            ResourceType::Stone => 's',
            ResourceType::Iron => 'i',
            ResourceType::Food => 'f',
            ResourceType::Water => 'w',

            // Agricultural
            ResourceType::Grain => 'g',
            ResourceType::Flax => 'x',
            ResourceType::Herbs => 'h',
            ResourceType::Cotton => 'c',

            // Animal
            ResourceType::Hides => 'H',
            ResourceType::Wool => 'W',
            ResourceType::Meat => 'm',
            ResourceType::Milk => 'M',
            ResourceType::Fish => '~',
            ResourceType::Honey => 'y',

            // Mineral
            ResourceType::Clay => 'C',
            ResourceType::Sand => 'd',
            ResourceType::Coal => 'o',

            // Processed
            ResourceType::Flour => 'F',
            ResourceType::Leather => 'L',
            ResourceType::Cloth => 'l',
            ResourceType::Linen => 'n',
            ResourceType::Glass => 'G',
            ResourceType::Bricks => 'B',
            ResourceType::Charcoal => 'k',
            ResourceType::Rope => 'r',
            ResourceType::Paper => 'p',
            ResourceType::Dye => 'D',

            // Finished Food
            ResourceType::Bread => 'b',
            ResourceType::Ale => 'a',
            ResourceType::Cheese => 'e',

            // Finished Items
            ResourceType::Clothing => 'T',
            ResourceType::Shoes => 'S',
            ResourceType::Tools => 'O',
            ResourceType::Weapons => 'w',
            ResourceType::Armor => 'A',
            ResourceType::Pottery => 'P',
            ResourceType::Furniture => 'R',
            ResourceType::Jewelry => 'J',
        }
    }

    /// Get color code for terminal rendering
    pub fn color_code(&self) -> &'static str {
        match self {
            // Basic - Original colors
            ResourceType::Wood => "\x1b[33m",      // Yellow/Brown
            ResourceType::Stone => "\x1b[37;1m",   // Bright White
            ResourceType::Iron => "\x1b[90m",      // Dark Gray
            ResourceType::Food => "\x1b[92m",      // Bright Green
            ResourceType::Water => "\x1b[96m",     // Bright Cyan (water)

            // Agricultural - Green shades
            ResourceType::Grain => "\x1b[93m",     // Bright Yellow (wheat)
            ResourceType::Flax => "\x1b[36m",      // Cyan
            ResourceType::Herbs => "\x1b[32m",     // Green
            ResourceType::Cotton => "\x1b[97m",    // Bright White

            // Animal - Brown/Red shades
            ResourceType::Hides => "\x1b[33m",     // Yellow/Brown
            ResourceType::Wool => "\x1b[37m",      // White
            ResourceType::Meat => "\x1b[31m",      // Red
            ResourceType::Milk => "\x1b[97m",      // Bright White
            ResourceType::Fish => "\x1b[96m",      // Bright Cyan
            ResourceType::Honey => "\x1b[93m",     // Bright Yellow

            // Mineral - Gray/Brown shades
            ResourceType::Clay => "\x1b[33m",      // Yellow/Brown
            ResourceType::Sand => "\x1b[93m",      // Bright Yellow
            ResourceType::Coal => "\x1b[90m",      // Dark Gray

            // Processed - Varied colors
            ResourceType::Flour => "\x1b[97m",     // Bright White
            ResourceType::Leather => "\x1b[33m",   // Yellow/Brown
            ResourceType::Cloth => "\x1b[36m",     // Cyan
            ResourceType::Linen => "\x1b[37m",     // White
            ResourceType::Glass => "\x1b[96m",     // Bright Cyan
            ResourceType::Bricks => "\x1b[31m",    // Red
            ResourceType::Charcoal => "\x1b[90m",  // Dark Gray
            ResourceType::Rope => "\x1b[33m",      // Yellow/Brown
            ResourceType::Paper => "\x1b[97m",     // Bright White
            ResourceType::Dye => "\x1b[35m",       // Magenta

            // Finished Food - Warm colors
            ResourceType::Bread => "\x1b[33m",     // Yellow/Brown
            ResourceType::Ale => "\x1b[93m",       // Bright Yellow
            ResourceType::Cheese => "\x1b[93m",    // Bright Yellow

            // Finished Items - Various
            ResourceType::Clothing => "\x1b[36m",  // Cyan
            ResourceType::Shoes => "\x1b[33m",     // Yellow/Brown
            ResourceType::Tools => "\x1b[37m",     // White
            ResourceType::Weapons => "\x1b[37;1m", // Bright White
            ResourceType::Armor => "\x1b[37;1m",   // Bright White
            ResourceType::Pottery => "\x1b[33m",   // Yellow/Brown
            ResourceType::Furniture => "\x1b[33m", // Yellow/Brown
            ResourceType::Jewelry => "\x1b[93m",   // Bright Yellow
        }
    }

    /// Get gather time per unit (in ticks)
    /// For raw materials: time to harvest/gather
    /// For processed/finished: time to craft (base time, modified by skill)
    pub fn gather_time(&self) -> u32 {
        match self {
            // Basic - gathering
            ResourceType::Wood => 20,
            ResourceType::Stone => 30,
            ResourceType::Iron => 40,
            ResourceType::Food => 15,
            ResourceType::Water => 5, // Very quick to drink/fill containers

            // Agricultural - farming/harvesting
            ResourceType::Grain => 25,
            ResourceType::Flax => 25,
            ResourceType::Herbs => 15,
            ResourceType::Cotton => 25,

            // Animal - from animals/butchering
            ResourceType::Hides => 30,
            ResourceType::Wool => 20,
            ResourceType::Meat => 25,
            ResourceType::Milk => 10,
            ResourceType::Fish => 30,
            ResourceType::Honey => 20,

            // Mineral - mining/gathering
            ResourceType::Clay => 20,
            ResourceType::Sand => 15,
            ResourceType::Coal => 35,

            // Processed - crafting time
            ResourceType::Flour => 10,      // Milling
            ResourceType::Leather => 40,    // Tanning (slow process)
            ResourceType::Cloth => 30,      // Weaving
            ResourceType::Linen => 30,      // Weaving
            ResourceType::Glass => 50,      // Glassblowing (difficult)
            ResourceType::Bricks => 25,     // Brick making
            ResourceType::Charcoal => 35,   // Charcoal burning
            ResourceType::Rope => 20,       // Rope making
            ResourceType::Paper => 30,      // Paper making
            ResourceType::Dye => 15,        // Dye making

            // Finished Food - preparation time
            ResourceType::Bread => 20,      // Baking
            ResourceType::Ale => 30,        // Brewing
            ResourceType::Cheese => 25,     // Cheese making

            // Finished Items - crafting time
            ResourceType::Clothing => 40,   // Tailoring
            ResourceType::Shoes => 35,      // Cobbling
            ResourceType::Tools => 45,      // Tool making
            ResourceType::Weapons => 60,    // Weapon crafting
            ResourceType::Armor => 70,      // Armor crafting
            ResourceType::Pottery => 30,    // Pottery making
            ResourceType::Furniture => 50,  // Furniture making
            ResourceType::Jewelry => 55,    // Jewelry crafting
        }
    }

    /// Check if this is a raw/harvestable resource (found in world)
    pub fn is_harvestable(&self) -> bool {
        matches!(
            self,
            ResourceType::Wood | ResourceType::Stone | ResourceType::Iron | ResourceType::Food |
            ResourceType::Water | // Water from rivers, wells, springs
            ResourceType::Grain | ResourceType::Flax | ResourceType::Herbs | ResourceType::Cotton |
            ResourceType::Clay | ResourceType::Sand | ResourceType::Coal |
            ResourceType::Fish | ResourceType::Honey
        )
    }

    /// Check if this is an animal product (requires animals)
    pub fn is_animal_product(&self) -> bool {
        matches!(
            self,
            ResourceType::Hides | ResourceType::Wool | ResourceType::Meat | ResourceType::Milk
        )
    }

    /// Check if this is a processed material (requires crafting)
    pub fn is_processed(&self) -> bool {
        matches!(
            self,
            ResourceType::Flour | ResourceType::Leather | ResourceType::Cloth |
            ResourceType::Linen | ResourceType::Glass | ResourceType::Bricks |
            ResourceType::Charcoal | ResourceType::Rope | ResourceType::Paper | ResourceType::Dye
        )
    }

    /// Check if this is a finished good (final product)
    pub fn is_finished_good(&self) -> bool {
        matches!(
            self,
            ResourceType::Bread | ResourceType::Ale | ResourceType::Cheese |
            ResourceType::Clothing | ResourceType::Shoes | ResourceType::Tools |
            ResourceType::Weapons | ResourceType::Armor | ResourceType::Pottery |
            ResourceType::Furniture | ResourceType::Jewelry
        )
    }

    /// Check if this is food/consumable
    pub fn is_consumable(&self) -> bool {
        matches!(
            self,
            ResourceType::Food | ResourceType::Bread | ResourceType::Ale |
            ResourceType::Cheese | ResourceType::Meat | ResourceType::Fish | ResourceType::Honey
        )
    }

    /// Get category description
    pub fn category(&self) -> &'static str {
        match self {
            ResourceType::Wood | ResourceType::Stone | ResourceType::Iron | ResourceType::Food | ResourceType::Water => "Basic Resource",
            ResourceType::Grain | ResourceType::Flax | ResourceType::Herbs | ResourceType::Cotton => "Agricultural",
            ResourceType::Hides | ResourceType::Wool | ResourceType::Meat | ResourceType::Milk => "Animal Product",
            ResourceType::Fish | ResourceType::Honey => "Animal Product",
            ResourceType::Clay | ResourceType::Sand | ResourceType::Coal => "Mineral",
            ResourceType::Flour | ResourceType::Leather | ResourceType::Cloth | ResourceType::Linen |
            ResourceType::Glass | ResourceType::Bricks | ResourceType::Charcoal | ResourceType::Rope |
            ResourceType::Paper | ResourceType::Dye => "Processed Material",
            ResourceType::Bread | ResourceType::Ale | ResourceType::Cheese => "Finished Food",
            ResourceType::Clothing | ResourceType::Shoes | ResourceType::Tools | ResourceType::Weapons |
            ResourceType::Armor | ResourceType::Pottery | ResourceType::Furniture | ResourceType::Jewelry => "Finished Good",
        }
    }
}

/// A resource node in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub resource_type: ResourceType,
    pub position: Position,
    pub amount: u32,
    pub max_amount: u32,
}

impl ResourceNode {
    pub fn new(resource_type: ResourceType, position: Position, amount: u32) -> Self {
        Self {
            resource_type,
            position,
            amount,
            max_amount: amount,
        }
    }

    /// Harvest resource from this node
    pub fn harvest(&mut self, amount: u32) -> u32 {
        let harvested = amount.min(self.amount);
        self.amount -= harvested;
        harvested
    }

    /// Check if node is depleted
    pub fn is_depleted(&self) -> bool {
        self.amount == 0
    }

    /// Get percentage remaining
    pub fn percentage_remaining(&self) -> f32 {
        if self.max_amount == 0 {
            return 0.0;
        }
        (self.amount as f32 / self.max_amount as f32) * 100.0
    }

    /// Whether this resource regrows on its own once harvested
    pub fn is_renewable(&self) -> bool {
        matches!(
            self.resource_type,
            ResourceType::Wood
                | ResourceType::Food
                | ResourceType::Grain
                | ResourceType::Herbs
                | ResourceType::Flax
                | ResourceType::Cotton
                | ResourceType::Honey
                | ResourceType::Fish
                // A river is not used up by the people drinking from it
                | ResourceType::Water
        )
    }

    /// Regenerate resources based on climate and weather conditions
    /// Returns the amount regenerated
    pub fn regenerate(&mut self, temperature: f32, precipitation: f32, season_modifier: f32) -> u32 {
        if self.amount >= self.max_amount {
            return 0; // Already at max
        }

        // Base regeneration rate per tick (0-1 units)
        let base_rate = match self.resource_type {
            // Renewable resources
            ResourceType::Wood => 0.01,       // Trees grow slowly
            ResourceType::Food => 0.05,       // Berries/fruits regenerate faster
            ResourceType::Grain => 0.03,      // Crops regenerate moderately
            ResourceType::Herbs => 0.04,      // Herbs grow quickly
            ResourceType::Flax => 0.03,
            ResourceType::Cotton => 0.03,
            ResourceType::Honey => 0.02,      // Bees produce honey steadily

            // Slow renewable
            ResourceType::Fish => 0.02,       // Fish populations regenerate

            // Rivers and lakes refill from rain and from what feeds them.
            //
            // Water used to regenerate at nothing at all and was not counted
            // as renewable, so every drink took a unit out of the world for
            // good and a lake that ran dry was deleted. Over fifteen thousand
            // ticks a world lost more than half its water that way, and the
            // settlements drinking from it died of thirst and then of hunger,
            // walking further and further for both.
            ResourceType::Water => 0.5,

            // Non-renewable (mineral resources don't regenerate)
            ResourceType::Stone |
            ResourceType::Iron |
            ResourceType::Clay |
            ResourceType::Sand |
            ResourceType::Coal => 0.0,

            // Processed/finished goods don't regenerate naturally
            _ => 0.0,
        };

        if base_rate == 0.0 {
            return 0;
        }

        // Apply temperature modifier (most resources prefer moderate temps)
        let temp_modifier = match self.resource_type {
            ResourceType::Food | ResourceType::Grain | ResourceType::Herbs => {
                // Plants prefer 15-25°C
                if temperature >= 15.0 && temperature <= 25.0 {
                    1.5 // Ideal conditions
                } else if temperature >= 5.0 && temperature <= 35.0 {
                    1.0 // Acceptable
                } else if temperature < -10.0 || temperature > 40.0 {
                    0.1 // Extreme temps slow growth severely
                } else {
                    0.5 // Suboptimal
                }
            },
            ResourceType::Wood => {
                // Trees are hardier
                if temperature >= -5.0 && temperature <= 30.0 {
                    1.0
                } else {
                    0.3
                }
            },
            ResourceType::Cotton => {
                // Cotton prefers warmer climates
                if temperature >= 20.0 && temperature <= 30.0 {
                    1.5
                } else if temperature >= 15.0 {
                    1.0
                } else {
                    0.3
                }
            },
            _ => 1.0, // No temperature preference
        };

        // Apply precipitation modifier (water availability)
        let precip_modifier = match self.resource_type {
            ResourceType::Food | ResourceType::Grain | ResourceType::Herbs | ResourceType::Flax => {
                // Most crops need moderate precipitation
                if precipitation >= 0.4 && precipitation <= 0.8 {
                    1.5 // Good rainfall
                } else if precipitation >= 0.2 {
                    1.0 // Adequate
                } else if precipitation < 0.1 {
                    0.2 // Drought
                } else {
                    0.7 // Too dry or too wet
                }
            },
            ResourceType::Wood => {
                // Trees need regular water
                if precipitation >= 0.3 {
                    1.2
                } else {
                    0.5
                }
            },
            ResourceType::Cotton => {
                // Cotton prefers drier conditions
                if precipitation >= 0.2 && precipitation <= 0.5 {
                    1.3
                } else if precipitation > 0.8 {
                    0.6 // Too wet
                } else {
                    0.8
                }
            },
            _ => 1.0,
        };

        // Calculate total regeneration
        let regen_amount = base_rate * temp_modifier * precip_modifier * season_modifier;
        let regen_units = (regen_amount * 100.0).round() as u32; // Convert to whole units

        // Add regenerated amount, capped at max
        let actual_regen = regen_units.min(self.max_amount - self.amount);
        self.amount += actual_regen;

        actual_regen
    }
}

/// Resource for tracking what's needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub amount: u32,
}

impl Resource {
    pub fn new(resource_type: ResourceType, amount: u32) -> Self {
        Self {
            resource_type,
            amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_node_creation() {
        let pos = Position::new(5, 5);
        let node = ResourceNode::new(ResourceType::Wood, pos, 100);

        assert_eq!(node.resource_type, ResourceType::Wood);
        assert_eq!(node.position, pos);
        assert_eq!(node.amount, 100);
        assert_eq!(node.max_amount, 100);
    }

    #[test]
    fn test_resource_harvest() {
        let pos = Position::new(5, 5);
        let mut node = ResourceNode::new(ResourceType::Wood, pos, 100);

        let harvested = node.harvest(30);
        assert_eq!(harvested, 30);
        assert_eq!(node.amount, 70);

        // Try to harvest more than available
        let harvested = node.harvest(100);
        assert_eq!(harvested, 70); // Only 70 left
        assert_eq!(node.amount, 0);
        assert!(node.is_depleted());
    }

    #[test]
    fn test_resource_percentage() {
        let pos = Position::new(5, 5);
        let mut node = ResourceNode::new(ResourceType::Stone, pos, 100);

        assert!((node.percentage_remaining() - 100.0).abs() < 0.1);

        node.harvest(50);
        assert!((node.percentage_remaining() - 50.0).abs() < 0.1);

        node.harvest(50);
        assert!((node.percentage_remaining() - 0.0).abs() < 0.1);
    }
}
