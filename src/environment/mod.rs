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
use std::collections::BTreeMap;
use crate::core::DriveType;

pub mod making;
pub mod verbs;
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
    AnimalSpawnConfig, GrazingWeather, terrain_to_climate_zone,
};
pub use biome::{BiomeType, Biome};
pub use weather::{Weather, WeatherType, WeatherGenerator, PrecipitationType};
pub use exposure::{ExposureType, ExposureStatus, ExposureProtection};
pub use seasons::{
    Season, SeasonalCalendar, DAYS_PER_SEASON, DAYS_PER_YEAR, TICKS_PER_DAY, TICKS_PER_YEAR,
};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    pub drive_changes: BTreeMap<DriveType, f32>,
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
    /// Stand your ground against an animal that is not being hunted.
    ///
    /// Distinct from `Hunt`, which is how an agent goes after food and skins
    /// and reads the Hunting skill. This is what an angry agent does to the
    /// thing that is frightening everybody: it reads MeleeCombat, it teaches
    /// the agent about fighting rather than about hunting, and it is worth
    /// doing on a full stomach.
    Fight {
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
    /// Build, fuel and light a fire where the agent is standing
    LightFire,
    /// Put carried food on a fire the agent is standing at
    Cook { food_type: String },
    /// Make a garment from materials the agent is carrying
    MakeClothing { garment: String },
    /// Put on a garment the agent is carrying
    WearClothing { garment: String },
    /// Break the grass where the agent stands into a field, and sow it
    TillSoil,
    /// Go over a standing field pulling weeds out of it and picking pests off
    /// it, which is most of what growing a crop actually consists of
    TendField,
    /// Help yourself to something of somebody else's
    TakeFrom { from: uuid::Uuid },
    /// Put ground between yourself and something, at a run
    FleeFrom { away_from: (i32, i32, i32) },
    /// Neither fight nor run, because neither is possible. The third answer
    /// to a threat, and the one nobody chooses on purpose
    Freeze,
    /// Look closely at something in the pack, and sometimes work out what it
    /// is for
    Examine { what: String },
    /// Take a tool out of the pack and into a hand, so it is there when the
    /// work starts rather than being dug out mid-job
    Equip { what: String },
    /// Put back into the pack whatever is occupying a hand
    Unequip { what: String },
    /// Lay food out to dry, or hang it over a fire to smoke. The only thing
    /// anybody here can do to make food outlast the week it was got in
    Dry { what: String },
    /// Boil sea or salt-marsh water down for what is in it. The one route to
    /// salt for a people with no flat and no seam, and the reason a coast is
    /// worth living on
    Boil,
    /// Rub salt into food, which keeps it about seven times as long as
    /// leaving it alone and does not need a week of sun
    Salt { what: String },
    /// Dig a pit in the ground here, to keep things in
    Excavate,
    /// Put something into the pit here and put the earth back over it
    Cover { what: String },
    /// Take something lying on the ground here into the pack
    PickUp { what: String },
    /// Set something down on the ground here
    PutDown { what: String },
    /// Ask somebody about a thing they are carrying that you have never seen
    /// the like of.
    ///
    /// The one way a discovery has ever had of leaving the head that made it.
    /// Everything else in this model is found out first-hand or watched being
    /// done; nothing anywhere let a man who had worked something out *tell*
    /// anybody. A settlement of forty could work the same thing out forty
    /// times over and be no further on.
    AskAbout { who: uuid::Uuid, what: String },
    /// Swap something there is too much of for something there is not enough
    /// of, with somebody who has the opposite problem
    Trade { with: uuid::Uuid },
    /// Hand something over, wanting nothing back
    GiveTo { to: uuid::Uuid },
    /// Give away food you need yourself, to somebody of your own who needs it
    /// more. The other half of laying down your life for them, and the half
    /// that happens more often than the fighting
    GoWithout { for_them: uuid::Uuid },
    /// Work a thing in the pack down into another thing with an edge or a
    /// hammer: smashing, cutting, scraping. What each wants in the hand is
    /// declared in the verb matrix and enforced there.
    Work { verb: String, to: String },
    /// Eat a piece of something growing that nobody has tried, and find out
    Taste,
    /// Attempt a known step with the wrong thing where one of the parts goes
    TrySwapping {
        instead_of_making: String,
        instead_of: String,
        put_in: String,
    },
    /// Lift a piece of a plant that is known to be good, to carry home
    TakeCutting,
    /// Put a carried cutting in the ground where the agent is standing
    PlantCutting,
    /// Tip whatever is spoiling in the pack onto the ground here
    SpreadMuck,
    /// Take what the run is carrying, from the reach the agent is standing at
    Fish,
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
            Action::Fight { .. } => Some(DriveType::Safety), // Driving a thing off, not eating it
            Action::Fish => Some(DriveType::Hunger), // A fish is a meal first
            Action::Tame { .. } => Some(DriveType::Utility), // Taming provides future utility
            Action::CollectAnimalProduct { .. } => Some(DriveType::Industry), // Resource gathering
            Action::HarvestPlant { .. } => Some(DriveType::Industry), // Resource gathering
            Action::SeekShelter => Some(DriveType::Safety), // Seeking safety from weather
            Action::LightFire => Some(DriveType::Sustenance), // A fire is for the food that goes on it
            Action::Cook { .. } => Some(DriveType::Sustenance), // Preparing food, not eating it
            Action::TillSoil => Some(DriveType::Sustenance), // A field is next year's food
            Action::TendField => Some(DriveType::Sustenance), // A field left alone is no field
            Action::TakeFrom { .. } => Some(DriveType::Utility),
            Action::FleeFrom { .. } => Some(DriveType::Safety),
            Action::Freeze => Some(DriveType::Safety),
            Action::Examine { .. } => Some(DriveType::Curiosity),
            Action::Equip { .. } => Some(DriveType::Utility),
            Action::Unequip { .. } => Some(DriveType::Utility),
            Action::Dry { .. } => Some(DriveType::Preparedness),
            Action::Boil => Some(DriveType::Preparedness),
            Action::Salt { .. } => Some(DriveType::Preparedness),
            Action::Excavate => Some(DriveType::Preparedness),
            Action::Cover { .. } => Some(DriveType::Preparedness),
            Action::PickUp { .. } => Some(DriveType::Utility),
            Action::PutDown { .. } => Some(DriveType::Utility),
            // Asking after a thing is curiosity, and it is only ever done
            // when nothing worse is pressing
            Action::AskAbout { .. } => Some(DriveType::Curiosity),
            Action::Trade { .. } => Some(DriveType::Utility), // A trade is how you get the thing
            Action::GiveTo { .. } => Some(DriveType::Social), // A gift is not
            Action::GoWithout { .. } => Some(DriveType::Protection),
            Action::Work { .. } => Some(DriveType::Utility),
            Action::Taste => Some(DriveType::Curiosity), // Nobody eats a strange plant for the nutrition
            Action::TrySwapping { .. } => Some(DriveType::Curiosity),
            Action::TakeCutting => Some(DriveType::Sustenance),
            Action::PlantCutting => Some(DriveType::Sustenance),
            Action::SpreadMuck => Some(DriveType::Sustenance),
            Action::MakeClothing { .. } => Some(DriveType::Shelter), // Clothing is shelter you carry
            Action::WearClothing { .. } => Some(DriveType::Shelter),
            Action::Move { .. } => None,
            Action::Wait => None,
        }
    }
}

impl ActionResult {
    pub fn success() -> Self {
        Self {
            success: true,
            drive_changes: BTreeMap::new(),
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
            drive_changes: BTreeMap::new(),
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
