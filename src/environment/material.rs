// src/environment/material.rs
//! Material property system for environment plugins.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::agents::Quality;

/// Tool tier required to harvest a material
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ToolTier {
    /// No tool required (hand gathering)
    None,
    /// Wooden tools
    Wooden,
    /// Stone tools
    Stone,
    /// Iron tools
    Iron,
    /// Diamond/high-tier tools
    Diamond,
    /// Special/magical tools
    Special,
}

/// Categories of tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolType {
    /// Axe for chopping
    Axe,
    /// Pickaxe for mining
    Pickaxe,
    /// Shovel for digging
    Shovel,
    /// Hoe for farming
    Hoe,
    /// Sword for combat
    Sword,
    /// Hands (no tool)
    Hand,
    /// Any tool works
    Any,
}

/// Physical state of a material
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialState {
    /// Solid material
    Solid,
    /// Liquid material
    Liquid,
    /// Gas material
    Gas,
    /// Plasma (rare)
    Plasma,
}

/// Categories of materials for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialCategory {
    /// Natural resources (wood, stone, ore)
    Natural,
    /// Processed materials (planks, ingots)
    Processed,
    /// Food items
    Food,
    /// Tools and equipment
    Tool,
    /// Building materials
    Building,
    /// Decorative items
    Decorative,
    /// Fuel sources
    Fuel,
    /// Liquids
    Liquid,
    /// Plants
    Plant,
    /// Containers (waterskins, bottles, buckets)
    Container,
    /// Clothing and armor
    Clothing,
}

/// Properties of a material in the environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    /// Unique identifier for this material
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Category
    pub category: MaterialCategory,
    /// Physical state
    pub state: MaterialState,

    // Harvesting properties
    /// Hardness (0.0 = very soft, 10.0 = very hard)
    pub hardness: f32,
    /// Tool required to harvest
    pub required_tool: ToolType,
    /// Tool tier required
    pub required_tier: ToolTier,
    /// Base time to harvest (in ticks)
    pub harvest_time: u32,
    /// Drop quantity when harvested
    pub drop_quantity: (u32, u32), // (min, max)

    // Material properties
    /// Durability when used as a tool (0 = infinite)
    pub durability: u32,
    /// Weight per unit (in kg)
    pub weight: f32,
    /// Max stack size
    pub stack_size: u32,
    /// Whether it can be used as fuel
    pub is_fuel: bool,
    /// Fuel burn time if is_fuel is true
    pub fuel_burn_time: u32,
    /// Whether it's edible
    pub is_edible: bool,
    /// Food value if edible
    pub food_value: f32,
    /// Whether it's flammable
    pub is_flammable: bool,
    /// Light level emitted (0-15)
    pub light_level: u8,

    // Quality
    /// Quality of this material/tool (affects output quality and effectiveness)
    pub quality: Quality,

    // Metallurgy properties
    /// Melting point in °C (None if doesn't melt)
    pub melting_point: Option<f32>,
    /// Can this material be worked cold (hammered without heat)?
    pub can_cold_work: bool,
    /// Temperature needed to work this material (forging temp in °C)
    pub workable_temp: Option<f32>,
    /// Is this an ore containing metal?
    pub is_ore: bool,
    /// What metal does this ore produce when smelted?
    pub ore_metal_id: Option<String>,
    /// Metal purity/yield from ore (0.0 to 1.0)
    pub ore_yield: f32,

    // Custom properties for plugin-specific data
    pub properties: HashMap<String, String>,
}

impl Material {
    /// Create a new material with default values
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            category: MaterialCategory::Natural,
            state: MaterialState::Solid,
            hardness: 1.0,
            required_tool: ToolType::Hand,
            required_tier: ToolTier::None,
            harvest_time: 20,
            drop_quantity: (1, 1),
            durability: 0,
            weight: 1.0, // Default 1kg per unit
            stack_size: 64,
            is_fuel: false,
            fuel_burn_time: 0,
            is_edible: false,
            food_value: 0.0,
            is_flammable: false,
            light_level: 0,
            quality: Quality::Basic,  // Default to Basic quality
            melting_point: None,
            can_cold_work: false,
            workable_temp: None,
            is_ore: false,
            ore_metal_id: None,
            ore_yield: 0.0,
            properties: HashMap::new(),
        }
    }



    /// Builder pattern methods
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    pub fn with_category(mut self, category: MaterialCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_state(mut self, state: MaterialState) -> Self {
        self.state = state;
        self
    }

    pub fn with_hardness(mut self, hardness: f32) -> Self {
        self.hardness = hardness;
        self
    }

    pub fn with_tool_requirement(mut self, tool: ToolType, tier: ToolTier) -> Self {
        self.required_tool = tool;
        self.required_tier = tier;
        self
    }

    pub fn with_harvest_time(mut self, time: u32) -> Self {
        self.harvest_time = time;
        self
    }

    pub fn with_drop_quantity(mut self, min: u32, max: u32) -> Self {
        self.drop_quantity = (min, max);
        self
    }

    pub fn with_durability(mut self, durability: u32) -> Self {
        self.durability = durability;
        self
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_stack_size(mut self, size: u32) -> Self {
        self.stack_size = size;
        self
    }

    pub fn as_fuel(mut self, burn_time: u32) -> Self {
        self.is_fuel = true;
        self.fuel_burn_time = burn_time;
        self
    }

    pub fn as_food(mut self, food_value: f32) -> Self {
        self.is_edible = true;
        self.food_value = food_value;
        self
    }

    pub fn flammable(mut self) -> Self {
        self.is_flammable = true;
        self
    }


    // Metallurgy builder methods
    pub fn with_melting_point(mut self, temp: f32) -> Self {
        self.melting_point = Some(temp);
        self
    }

    pub fn with_cold_working(mut self) -> Self {
        self.can_cold_work = true;
        self
    }

    pub fn with_workable_temp(mut self, temp: f32) -> Self {
        self.workable_temp = Some(temp);
        self
    }

    pub fn as_ore(mut self, metal_id: String, yield_percent: f32) -> Self {
        self.is_ore = true;
        self.ore_metal_id = Some(metal_id);
        self.ore_yield = yield_percent;
        self
    }

    pub fn with_property(mut self, key: String, value: String) -> Self {
        self.properties.insert(key, value);
        self
    }

    /// Check if a tool can harvest this material
    pub fn can_harvest_with(&self, tool: ToolType, tier: ToolTier) -> bool {
        // Check tool type
        let tool_ok = match self.required_tool {
            ToolType::Any => true,
            ToolType::Hand => true, // Hand can always try
            required => tool == required || tool == ToolType::Any,
        };

        // Check tool tier
        let tier_ok = tier >= self.required_tier;

        tool_ok && tier_ok
    }

    /// Calculate effective harvest time with a given tool
    pub fn effective_harvest_time(&self, tool: ToolType, tier: ToolTier) -> u32 {
        if !self.can_harvest_with(tool, tier) {
            return self.harvest_time * 5; // Much slower with wrong tool
        }

        // Faster with better tier
        let tier_multiplier = match tier {
            ToolTier::None => 1.0,
            ToolTier::Wooden => 0.8,
            ToolTier::Stone => 0.6,
            ToolTier::Iron => 0.4,
            ToolTier::Diamond => 0.2,
            ToolTier::Special => 0.1,
        };

        (self.harvest_time as f32 * tier_multiplier).max(1.0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_creation() {
        let material = Material::new("stone".to_string(), "Stone".to_string())
            .with_hardness(3.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Wooden)
            .with_harvest_time(30);

        assert_eq!(material.id, "stone");
        assert_eq!(material.hardness, 3.0);
        assert_eq!(material.required_tool, ToolType::Pickaxe);
    }

    #[test]
    fn test_can_harvest_with() {
        let stone = Material::new("stone".to_string(), "Stone".to_string())
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Wooden);

        assert!(stone.can_harvest_with(ToolType::Pickaxe, ToolTier::Wooden));
        assert!(stone.can_harvest_with(ToolType::Pickaxe, ToolTier::Iron));
        assert!(!stone.can_harvest_with(ToolType::Axe, ToolTier::Wooden));
        assert!(!stone.can_harvest_with(ToolType::Pickaxe, ToolTier::None));
    }

    #[test]
    fn test_effective_harvest_time() {
        let stone = Material::new("stone".to_string(), "Stone".to_string())
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Wooden)
            .with_harvest_time(100);

        // Correct tool, wooden tier
        assert_eq!(stone.effective_harvest_time(ToolType::Pickaxe, ToolTier::Wooden), 80);

        // Correct tool, iron tier (faster)
        assert_eq!(stone.effective_harvest_time(ToolType::Pickaxe, ToolTier::Iron), 40);

        // Wrong tool (much slower)
        assert_eq!(stone.effective_harvest_time(ToolType::Axe, ToolTier::Iron), 500);
    }

    #[test]
    fn test_food_material() {
        let apple = Material::new("apple".to_string(), "Apple".to_string())
            .as_food(4.0)
            .with_category(MaterialCategory::Food);

        assert!(apple.is_edible);
        assert_eq!(apple.food_value, 4.0);
    }

    #[test]
    fn test_fuel_material() {
        let coal = Material::new("coal".to_string(), "Coal".to_string())
            .as_fuel(1600)
            .with_category(MaterialCategory::Fuel);

        assert!(coal.is_fuel);
        assert_eq!(coal.fuel_burn_time, 1600);
    }
}
