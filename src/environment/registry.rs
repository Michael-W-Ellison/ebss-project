// src/environment/registry.rs
//! Plugin registry for managing environment plugins.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use super::{EnvironmentPlugin, EnvironmentError, EnvironmentResult, PluginConfig};

/// Registry for managing environment plugins
///
/// The registry maintains a collection of loaded plugins and provides
/// access to them by ID. Plugins can be registered, retrieved, and
/// managed through this interface.
pub struct PluginRegistry {
    plugins: HashMap<String, Arc<RwLock<Box<dyn EnvironmentPlugin>>>>,
    active_plugin: Option<String>,
}

impl PluginRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            active_plugin: None,
        }
    }

    /// Register a new plugin
    ///
    /// # Arguments
    /// * `plugin` - The plugin to register
    ///
    /// # Returns
    /// The plugin ID if successful
    pub fn register(
        &mut self,
        plugin: Box<dyn EnvironmentPlugin>,
    ) -> EnvironmentResult<String> {
        let plugin_id = plugin.metadata().id.clone();

        if self.plugins.contains_key(&plugin_id) {
            return Err(EnvironmentError::Other(format!(
                "Plugin '{}' is already registered",
                plugin_id
            )));
        }

        self.plugins
            .insert(plugin_id.clone(), Arc::new(RwLock::new(plugin)));

        Ok(plugin_id)
    }

    /// Get a reference to a plugin by ID
    pub fn get(&self, plugin_id: &str) -> Option<Arc<RwLock<Box<dyn EnvironmentPlugin>>>> {
        self.plugins.get(plugin_id).cloned()
    }

    /// Set the active plugin
    pub fn set_active(&mut self, plugin_id: &str) -> EnvironmentResult<()> {
        if !self.plugins.contains_key(plugin_id) {
            return Err(EnvironmentError::PluginNotFound(plugin_id.to_string()));
        }

        self.active_plugin = Some(plugin_id.to_string());
        Ok(())
    }

    /// Get the active plugin
    pub fn get_active(&self) -> Option<Arc<RwLock<Box<dyn EnvironmentPlugin>>>> {
        self.active_plugin
            .as_ref()
            .and_then(|id| self.plugins.get(id).cloned())
    }

    /// Get the active plugin ID
    pub fn get_active_id(&self) -> Option<&str> {
        self.active_plugin.as_deref()
    }

    /// Check if a plugin is registered
    pub fn has_plugin(&self, plugin_id: &str) -> bool {
        self.plugins.contains_key(plugin_id)
    }

    /// Get all registered plugin IDs
    pub fn plugin_ids(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Remove a plugin from the registry
    pub fn unregister(&mut self, plugin_id: &str) -> EnvironmentResult<()> {
        if !self.plugins.contains_key(plugin_id) {
            return Err(EnvironmentError::PluginNotFound(plugin_id.to_string()));
        }

        // Don't allow removing the active plugin
        if self.active_plugin.as_deref() == Some(plugin_id) {
            return Err(EnvironmentError::Other(
                "Cannot unregister active plugin".to_string(),
            ));
        }

        self.plugins.remove(plugin_id);
        Ok(())
    }

    /// Clear all plugins
    pub fn clear(&mut self) {
        self.plugins.clear();
        self.active_plugin = None;
    }

    /// Get the number of registered plugins
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Register and activate a plugin in one step
    pub fn register_and_activate(
        &mut self,
        mut plugin: Box<dyn EnvironmentPlugin>,
        config: PluginConfig,
    ) -> EnvironmentResult<String> {
        // Initialize the plugin
        plugin.initialize(config)?;

        // Register it
        let plugin_id = self.register(plugin)?;

        // Set it as active
        self.set_active(&plugin_id)?;

        Ok(plugin_id)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global plugin registry instance
static mut GLOBAL_REGISTRY: Option<PluginRegistry> = None;
static REGISTRY_INIT: std::sync::Once = std::sync::Once::new();

/// Get the global plugin registry
#[allow(static_mut_refs)]
pub fn global_registry() -> &'static mut PluginRegistry {
    unsafe {
        REGISTRY_INIT.call_once(|| {
            GLOBAL_REGISTRY = Some(PluginRegistry::new());
        });
        GLOBAL_REGISTRY.as_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::*;
    use std::any::Any;

    // Mock plugin for testing
    struct MockPlugin {
        metadata: PluginMetadata,
        world_state: WorldState,
        materials: Vec<Material>,
        actions: Vec<Action>,
        recipe_book: RecipeBook,
    }

    impl MockPlugin {
        fn new(id: String) -> Self {
            Self {
                metadata: PluginMetadata::new(id, "Mock Plugin".to_string(), "1.0.0".to_string()),
                world_state: WorldState::new(0),
                materials: Vec::new(),
                actions: Vec::new(),
                recipe_book: RecipeBook::new(),
            }
        }
    }

    impl EnvironmentPlugin for MockPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        fn initialize(&mut self, config: PluginConfig) -> EnvironmentResult<()> {
            self.world_state.seed = config.seed;
            Ok(())
        }

        fn get_materials(&self) -> Vec<&Material> {
            self.materials.iter().collect()
        }

        fn get_material(&self, _material_id: &str) -> Option<&Material> {
            None
        }

        fn get_actions(&self) -> Vec<&Action> {
            self.actions.iter().collect()
        }

        fn get_action(&self, _action_id: &str) -> Option<&Action> {
            None
        }

        fn get_recipe_book(&self) -> &RecipeBook {
            &self.recipe_book
        }

        fn get_world_state(&self) -> &WorldState {
            &self.world_state
        }

        fn execute_action(
            &mut self,
            _action: &Action,
            _context: ActionContext,
        ) -> EnvironmentResult<ActionResult> {
            Ok(ActionResult::success())
        }

        fn tick(&mut self) {
            self.world_state.tick += 1;
        }

        fn get_material_at(&self, _position: Position) -> Option<&Material> {
            None
        }

        fn is_walkable(&self, _position: Position) -> bool {
            true
        }

        fn is_valid_position(&self, _position: Position) -> bool {
            true
        }

        fn find_nearby_materials(
            &self,
            _position: Position,
            _material_id: &str,
            _radius: f32,
        ) -> Vec<Position> {
            Vec::new()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(registry.get_active().is_none());
    }

    #[test]
    fn test_register_plugin() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(MockPlugin::new("test".to_string()));

        let result = registry.register(plugin);
        assert!(result.is_ok());
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry = PluginRegistry::new();
        let plugin1 = Box::new(MockPlugin::new("test".to_string()));
        let plugin2 = Box::new(MockPlugin::new("test".to_string()));

        registry.register(plugin1).unwrap();
        let result = registry.register(plugin2);

        assert!(result.is_err());
    }

    #[test]
    fn test_set_active_plugin() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(MockPlugin::new("test".to_string()));

        registry.register(plugin).unwrap();
        let result = registry.set_active("test");

        assert!(result.is_ok());
        assert_eq!(registry.get_active_id(), Some("test"));
    }

    #[test]
    fn test_set_active_nonexistent() {
        let mut registry = PluginRegistry::new();
        let result = registry.set_active("nonexistent");

        assert!(result.is_err());
    }

    #[test]
    fn test_get_plugin() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(MockPlugin::new("test".to_string()));

        registry.register(plugin).unwrap();
        let retrieved = registry.get("test");

        assert!(retrieved.is_some());
    }

    #[test]
    fn test_unregister_plugin() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(MockPlugin::new("test".to_string()));

        registry.register(plugin).unwrap();
        let result = registry.unregister("test");

        assert!(result.is_ok());
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_cannot_unregister_active() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(MockPlugin::new("test".to_string()));

        registry.register(plugin).unwrap();
        registry.set_active("test").unwrap();
        let result = registry.unregister("test");

        assert!(result.is_err());
    }

    #[test]
    fn test_register_and_activate() {
        let mut registry = PluginRegistry::new();
        let plugin = Box::new(MockPlugin::new("test".to_string()));
        let config = PluginConfig::new(12345);

        let result = registry.register_and_activate(plugin, config);

        assert!(result.is_ok());
        assert_eq!(registry.get_active_id(), Some("test"));
    }
}
