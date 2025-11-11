// src/environment/plugin.rs
//! Environment plugin trait and related types.

use std::any::Any;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::{
    Material, Action, CraftingTemplate, ActionContext, ActionResult,
    Position, EnvironmentResult, RecipeBook,
};

/// Metadata about an environment plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique identifier for this plugin
    pub id: String,
    /// Display name
    pub name: String,
    /// Version string
    pub version: String,
    /// Author(s)
    pub author: String,
    /// Description
    pub description: String,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl PluginMetadata {
    pub fn new(id: String, name: String, version: String) -> Self {
        Self {
            id,
            name,
            version,
            author: String::new(),
            description: String::new(),
            tags: Vec::new(),
        }
    }
}

/// World state that plugins manage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// World seed for generation
    pub seed: u64,
    /// Simulation tick count
    pub tick: u64,
    /// Time of day (0.0 to 1.0, where 0.5 is noon)
    pub time_of_day: f32,
    /// Current weather
    pub weather: String,
    /// Temperature
    pub temperature: f32,
    /// Custom state data (plugin-specific)
    pub custom_data: HashMap<String, String>,
}

impl WorldState {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            tick: 0,
            time_of_day: 0.0,
            weather: "clear".to_string(),
            temperature: 20.0,
            custom_data: HashMap::new(),
        }
    }

    pub fn advance_tick(&mut self, time_rate: f32) {
        self.tick += 1;
        self.time_of_day = (self.time_of_day + time_rate).rem_euclid(1.0);
    }
}

/// Configuration for initializing a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// World seed
    pub seed: u64,
    /// World size parameters
    pub world_size: (i32, i32, i32),
    /// Difficulty level (0.0 to 1.0)
    pub difficulty: f32,
    /// Custom configuration (plugin-specific)
    pub custom_config: HashMap<String, String>,
}

impl PluginConfig {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            world_size: (256, 256, 128),
            difficulty: 0.5,
            custom_config: HashMap::new(),
        }
    }
}

/// Trait that all environment plugins must implement
///
/// This trait defines the interface for environment plugins. Plugins control:
/// - Materials available in the world
/// - Actions agents can perform
/// - Crafting recipes
/// - World generation and state
/// - Action execution and results
pub trait EnvironmentPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin with configuration
    fn initialize(&mut self, config: PluginConfig) -> EnvironmentResult<()>;

    /// Get all materials in this environment
    fn get_materials(&self) -> Vec<&Material>;

    /// Get a specific material by ID
    fn get_material(&self, material_id: &str) -> Option<&Material>;

    /// Get all available actions
    fn get_actions(&self) -> Vec<&Action>;

    /// Get a specific action by ID
    fn get_action(&self, action_id: &str) -> Option<&Action>;

    /// Get the recipe book
    fn get_recipe_book(&self) -> &RecipeBook;

    /// Get current world state
    fn get_world_state(&self) -> &WorldState;

    /// Execute an action in the environment
    fn execute_action(
        &mut self,
        action: &Action,
        context: ActionContext,
    ) -> EnvironmentResult<ActionResult>;

    /// Update world state (called each tick)
    fn tick(&mut self);

    /// Get material at a specific position
    fn get_material_at(&self, position: Position) -> Option<&Material>;

    /// Check if a position is walkable
    fn is_walkable(&self, position: Position) -> bool;

    /// Check if a position is within world bounds
    fn is_valid_position(&self, position: Position) -> bool;

    /// Find nearby materials of a specific type
    fn find_nearby_materials(
        &self,
        position: Position,
        material_id: &str,
        radius: f32,
    ) -> Vec<Position>;

    /// Get plugin-specific data (for advanced use cases)
    fn as_any(&self) -> &dyn Any;

    /// Get mutable plugin-specific data
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Helper trait for downcasting plugin references
pub trait EnvironmentPluginExt: EnvironmentPlugin {
    /// Downcast to a concrete plugin type
    fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    /// Downcast to a mutable concrete plugin type
    fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

impl<T: EnvironmentPlugin + ?Sized> EnvironmentPluginExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata() {
        let meta = PluginMetadata::new(
            "test_plugin".to_string(),
            "Test Plugin".to_string(),
            "1.0.0".to_string(),
        );

        assert_eq!(meta.id, "test_plugin");
        assert_eq!(meta.name, "Test Plugin");
        assert_eq!(meta.version, "1.0.0");
    }

    #[test]
    fn test_world_state() {
        let mut state = WorldState::new(12345);
        assert_eq!(state.tick, 0);
        assert_eq!(state.time_of_day, 0.0);

        state.advance_tick(0.01);
        assert_eq!(state.tick, 1);
        assert_eq!(state.time_of_day, 0.01);
    }

    #[test]
    fn test_plugin_config() {
        let config = PluginConfig::new(54321);
        assert_eq!(config.seed, 54321);
        assert_eq!(config.world_size, (256, 256, 128));
        assert_eq!(config.difficulty, 0.5);
    }
}
