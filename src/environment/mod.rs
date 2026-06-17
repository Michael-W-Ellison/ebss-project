// src/environment/mod.rs
//! Environment abstraction layer for different world types.
//!
//! This module provides a plugin architecture for implementing different environment
//! types (e.g., Minecraft-style survival, Dwarf Fortress, medieval simulations).
//!
//! # Architecture
//!
//! - `EnvironmentPlugin`: Trait that all environment plugins must implement
//! - `Material`: Defines properties of materials (hardness, durability, tool requirements)
//! - `Action`: Represents agent actions in the environment
//! - `CraftingTemplate`: Defines crafting recipes and requirements
//! - `PluginRegistry`: Manages loaded plugins and provides access to them

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::core::DriveType;

mod material;
mod action;
mod crafting;
mod plugin;
mod registry;
mod structure;
pub mod technology;
mod heat_source;
pub mod smelting;
pub mod clothing_recipes;
pub mod flora;
pub mod fauna;
pub mod biome;
pub mod weather;
pub mod exposure;
pub mod seasons;
mod minecraft_survival;

pub use material::*;
pub use minecraft_survival::MinecraftSurvivalPlugin;
pub use action::*;
pub use crafting::*;
pub use plugin::*;
pub use registry::*;
pub use structure::*;
pub use technology::*;
pub use heat_source::*;
pub use flora::{PlantSpecies, FloraRegistry, ClimateZone, GrowthStage, PlantSize, Plant, PlantManager, PlantDrop};
pub use fauna::{
    AnimalSpecies, FaunaRegistry, AnimalBehavior, DietType, AnimalSize,
    Animal, AnimalState, AnimalManager, AnimalDrop, AnimalProduct,
    AnimalSpawnConfig, terrain_to_climate_zone,
};
pub use biome::{BiomeType, Biome};
pub use weather::{Weather, WeatherType, WeatherGenerator, PrecipitationType};
pub use exposure::{ExposureType, ExposureStatus, ExposureProtection};
pub use seasons::{Season, SeasonalCalendar};

/// Result type for environment operations
pub type EnvironmentResult<T> = Result<T, EnvironmentError>;

/// Errors that can occur in environment operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvironmentError {
    /// Action failed due to missing requirements
    RequirementsNotMet(String),
    /// Invalid action for current state
    InvalidAction(String),
    /// Material not found
    MaterialNotFound(String),
    /// Recipe not found
    RecipeNotFound(String),
    /// Plugin not found
    PluginNotFound(String),
    /// Plugin initialization failed
    PluginInitFailed(String),
    /// Generic error
    Other(String),
}

impl std::fmt::Display for EnvironmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvironmentError::RequirementsNotMet(msg) => write!(f, "Requirements not met: {}", msg),
            EnvironmentError::InvalidAction(msg) => write!(f, "Invalid action: {}", msg),
            EnvironmentError::MaterialNotFound(msg) => write!(f, "Material not found: {}", msg),
            EnvironmentError::RecipeNotFound(msg) => write!(f, "Recipe not found: {}", msg),
            EnvironmentError::PluginNotFound(msg) => write!(f, "Plugin not found: {}", msg),
            EnvironmentError::PluginInitFailed(msg) => write!(f, "Plugin init failed: {}", msg),
            EnvironmentError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for EnvironmentError {}

/// Position in 3D space
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Position {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        let dz = (self.z - other.z) as f32;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

impl From<(i32, i32, i32)> for Position {
    fn from((x, y, z): (i32, i32, i32)) -> Self {
        Self { x, y, z }
    }
}

/// Represents the result of an action in the environment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,
    /// Drives that were affected and by how much (positive = satisfied, negative = increased)
    pub drive_changes: HashMap<DriveType, f32>,
    /// Items produced or obtained
    pub items_gained: Vec<ItemStack>,
    /// Items consumed or lost
    pub items_consumed: Vec<ItemStack>,
    /// Experience gained
    pub experience: f32,
    /// Energy cost
    pub energy_cost: f32,
    /// Overall drive satisfaction from this action
    pub drive_satisfaction: f32,
    /// Message describing what happened
    pub message: Option<String>,
}

/// Actions that agents can perform in the environment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Move to a specific position
    Move { target: (i32, i32, i32) },
    /// Gather a resource
    Gather { resource_type: String },
    /// Consume food
    Eat { food_type: String },
    /// Sleep to restore energy
    Sleep { duration: u32 },
    /// Build a structure
    Build { structure_type: String, position: (i32, i32, i32) },
    /// Craft an item
    Craft { item_type: String },
    /// Store items in communal storehouse
    Store { item_type: String, amount: u32 },
    /// Retrieve items from communal storehouse
    Retrieve { item_type: String, amount: u32 },
    /// Explore a new area
    Explore { direction: (i32, i32, i32) },
    /// Interact with another agent
    Socialize { target_agent_id: uuid::Uuid },
    /// Attack another agent
    Attack {
        target_agent_id: uuid::Uuid,
        weapon: Option<String>,  // Optional weapon (None = unarmed)
    },
    /// Hunt a wild animal
    Hunt {
        animal_id: uuid::Uuid,
        weapon: Option<String>,
    },
    /// Tame a wild animal
    Tame {
        animal_id: uuid::Uuid,
        food_type: Option<String>,
    },
    /// Collect product from domesticated animal (eggs, milk, wool)
    CollectAnimalProduct {
        animal_id: uuid::Uuid,
    },
    /// Harvest a plant (crop or wild)
    HarvestPlant {
        plant_id: uuid::Uuid,
    },
    /// Share information/gossip with another agent
    ShareInformation {
        target_agent_id: uuid::Uuid,
    },
    /// Attempt to mate with another agent
    Mate {
        target_agent_id: uuid::Uuid,
    },
    /// Mount a transport (horse, camel, etc.)
    Mount {
        transport_id: uuid::Uuid,
    },
    /// Dismount from current mount
    Dismount,
    /// Seek shelter from dangerous weather
    SeekShelter,
    /// Repair equipment or tools
    Repair {
        slot: String, // Equipment slot to repair ("main_hand", "torso", etc.)
    },
    /// Wait/idle
    Wait,
}

impl Action {
    /// Get which drive this action primarily satisfies
    pub fn primary_drive(&self) -> Option<DriveType> {
        match self {
            Action::Eat { .. } => Some(DriveType::Hunger),
            Action::Sleep { .. } => Some(DriveType::Rest),
            Action::Build { .. } => Some(DriveType::Construction),
            Action::Gather { .. } => Some(DriveType::Industry),
            Action::Craft { .. } => Some(DriveType::Utility),
            Action::Store { .. } => Some(DriveType::Preparedness),
            Action::Retrieve { .. } => Some(DriveType::Preparedness),
            Action::Explore { .. } => Some(DriveType::Curiosity),
            Action::Socialize { .. } => Some(DriveType::Social),
            Action::ShareInformation { .. } => Some(DriveType::Social), // Information sharing is social
            Action::Mate { .. } => Some(DriveType::Reproduction), // Mating satisfies reproduction drive
            Action::Mount { .. } => Some(DriveType::Utility), // Mounting provides travel utility
            Action::Dismount => Some(DriveType::Utility), // Dismounting when needed
            Action::Attack { .. } => Some(DriveType::Safety), // Defense/aggression
            Action::Hunt { .. } => Some(DriveType::Hunger), // Hunting for food
            Action::Tame { .. } => Some(DriveType::Utility), // Taming provides future utility
            Action::CollectAnimalProduct { .. } => Some(DriveType::Industry), // Resource gathering
            Action::HarvestPlant { .. } => Some(DriveType::Industry), // Resource gathering
            Action::SeekShelter => Some(DriveType::Safety), // Seeking safety from weather
            Action::Repair { .. } => Some(DriveType::Utility), // Maintaining equipment
            Action::Move { .. } => None,
            Action::Wait => None,
        }
    }
}

impl ActionResult {
    pub fn success() -> Self {
        Self {
            success: true,
            drive_changes: HashMap::new(),
            items_gained: Vec::new(),
            items_consumed: Vec::new(),
            experience: 0.0,
            energy_cost: 0.0,
            drive_satisfaction: 0.0,
            message: None,
        }
    }

    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            drive_changes: HashMap::new(),
            items_gained: Vec::new(),
            items_consumed: Vec::new(),
            experience: 0.0,
            energy_cost: 0.0,
            drive_satisfaction: 0.0,
            message: Some(message),
        }
    }

    pub fn with_drive_change(mut self, drive: DriveType, amount: f32) -> Self {
        self.drive_changes.insert(drive, amount);
        self
    }

    pub fn with_item_gained(mut self, item: ItemStack) -> Self {
        self.items_gained.push(item);
        self
    }

    pub fn with_item_consumed(mut self, item: ItemStack) -> Self {
        self.items_consumed.push(item);
        self
    }

    pub fn with_experience(mut self, exp: f32) -> Self {
        self.experience = exp;
        self
    }

    pub fn with_energy_cost(mut self, cost: f32) -> Self {
        self.energy_cost = cost;
        self
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

/// Stack of items
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStack {
    pub material_id: String,
    pub quantity: u32,
}

impl ItemStack {
    pub fn new(material_id: String, quantity: u32) -> Self {
        Self { material_id, quantity }
    }
}

#[cfg(test)]
#[path = "tests/technology_progression_tests.rs"]
mod technology_progression_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_distance() {
        let p1 = Position::new(0, 0, 0);
        let p2 = Position::new(3, 4, 0);
        assert_eq!(p1.distance_to(&p2), 5.0);
    }

    #[test]
    fn test_position_from_tuple() {
        let pos: Position = (1, 2, 3).into();
        assert_eq!(pos.x, 1);
        assert_eq!(pos.y, 2);
        assert_eq!(pos.z, 3);
    }

    #[test]
    fn test_action_result_builder() {
        let result = ActionResult::success()
            .with_drive_change(DriveType::Hunger, -0.5)
            .with_experience(10.0)
            .with_message("Ate food".to_string());

        assert!(result.success);
        assert_eq!(result.drive_changes.get(&DriveType::Hunger), Some(&-0.5));
        assert_eq!(result.experience, 10.0);
    }
}
