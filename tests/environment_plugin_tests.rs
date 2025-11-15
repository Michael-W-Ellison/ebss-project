// tests/environment_plugin_tests.rs
//! Integration tests for the environment plugin system.

use ebss::environment::*;
use ebss::core::DriveType;
use std::any::Any;
use std::collections::HashMap;

// Mock plugin for testing
struct TestPlugin {
    metadata: PluginMetadata,
    world_state: WorldState,
    materials: HashMap<String, Material>,
    actions: HashMap<String, Action>,
    recipe_book: RecipeBook,
    initialized: bool,
}

impl TestPlugin {
    fn new() -> Self {
        let mut plugin = Self {
            metadata: PluginMetadata::new(
                "test_plugin".to_string(),
                "Test Plugin".to_string(),
                "1.0.0".to_string(),
            ),
            world_state: WorldState::new(0),
            materials: HashMap::new(),
            actions: HashMap::new(),
            recipe_book: RecipeBook::new(),
            initialized: false,
        };

        // Add a test material
        let wood = Material::new("wood".to_string(), "Wood".to_string())
            .with_hardness(2.0)
            .with_tool_requirement(ToolType::Axe, ToolTier::None)
            .as_fuel(300);
        plugin.materials.insert("wood".to_string(), wood);

        // Add a test action (using new Action enum)
        let harvest = Action::Gather { resource_type: "wood".to_string() };
        plugin.actions.insert("harvest".to_string(), harvest);

        // Add a test recipe
        let recipe = CraftingTemplate::new("planks".to_string(), "Planks".to_string())
            .with_input(Ingredient::new("wood".to_string(), 1))
            .with_output(CraftingOutput::new("planks".to_string(), 4));
        plugin.recipe_book.add_recipe(recipe);

        plugin
    }
}

impl EnvironmentPlugin for TestPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self, config: PluginConfig) -> EnvironmentResult<()> {
        self.world_state.seed = config.seed;
        self.initialized = true;
        Ok(())
    }

    fn get_materials(&self) -> Vec<&Material> {
        self.materials.values().collect()
    }

    fn get_material(&self, material_id: &str) -> Option<&Material> {
        self.materials.get(material_id)
    }

    fn get_actions(&self) -> Vec<&Action> {
        self.actions.values().collect()
    }

    fn get_action(&self, action_id: &str) -> Option<&Action> {
        self.actions.get(action_id)
    }

    fn get_recipe_book(&self) -> &RecipeBook {
        &self.recipe_book
    }

    fn get_world_state(&self) -> &WorldState {
        &self.world_state
    }

    fn execute_action(
        &mut self,
        action: &Action,
        _context: ActionContext,
    ) -> EnvironmentResult<ActionResult> {
        // Map action to result based on action type
        let mut result = ActionResult::success()
            .with_energy_cost(5.0);

        // Add drive changes based on action primary drive
        if let Some(drive) = action.primary_drive() {
            result = result.with_drive_change(drive, -0.1);
        }

        Ok(result)
    }

    fn tick(&mut self) {
        self.world_state.advance_tick(0.001);
    }

    fn get_material_at(&self, _position: Position) -> Option<&Material> {
        None
    }

    fn is_walkable(&self, _position: Position) -> bool {
        true
    }

    fn is_valid_position(&self, position: Position) -> bool {
        position.x >= -128 && position.x < 128
            && position.z >= -128 && position.z < 128
            && position.y >= 0 && position.y < 256
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
fn test_plugin_registration() {
    let mut registry = PluginRegistry::new();
    let plugin = Box::new(TestPlugin::new());

    let result = registry.register(plugin);
    assert!(result.is_ok());
    assert_eq!(registry.count(), 1);
}

#[test]
fn test_plugin_initialization() {
    let mut plugin = TestPlugin::new();
    let config = PluginConfig::new(12345);

    let result = plugin.initialize(config);
    assert!(result.is_ok());
    assert!(plugin.initialized);
    assert_eq!(plugin.world_state.seed, 12345);
}

#[test]
fn test_plugin_materials() {
    let plugin = TestPlugin::new();

    let materials = plugin.get_materials();
    assert_eq!(materials.len(), 1);

    let wood = plugin.get_material("wood");
    assert!(wood.is_some());
    assert_eq!(wood.unwrap().hardness, 2.0);
}

#[test]
fn test_plugin_actions() {
    let plugin = TestPlugin::new();

    let actions = plugin.get_actions();
    assert_eq!(actions.len(), 1);

    let harvest = plugin.get_action("harvest");
    assert!(harvest.is_some());
    // Action is now an enum, so we can't access effects field directly
    // assert_eq!(harvest.unwrap().effects.energy_cost, 5.0);
}

#[test]
fn test_plugin_recipes() {
    let plugin = TestPlugin::new();
    let book = plugin.get_recipe_book();

    let recipe = book.get_recipe("planks");
    assert!(recipe.is_some());
    assert_eq!(recipe.unwrap().inputs.len(), 1);
    assert_eq!(recipe.unwrap().outputs.len(), 1);
}

#[test]
fn test_action_execution() {
    let mut plugin = TestPlugin::new();
    let config = PluginConfig::new(0);
    plugin.initialize(config).unwrap();

    let action = plugin.get_action("harvest").unwrap().clone();
    let context = ActionContext::new("agent_123".to_string(), Position::new(0, 0, 0));

    let result = plugin.execute_action(&action, context);
    assert!(result.is_ok());

    let action_result = result.unwrap();
    assert!(action_result.success);
    assert_eq!(action_result.energy_cost, 5.0);
    assert_eq!(
        action_result.drive_changes.get(&DriveType::Industry),
        Some(&-0.1)
    );
}

#[test]
fn test_world_tick() {
    let mut plugin = TestPlugin::new();
    let config = PluginConfig::new(0);
    plugin.initialize(config).unwrap();

    let initial_tick = plugin.world_state.tick;
    plugin.tick();
    assert_eq!(plugin.world_state.tick, initial_tick + 1);
}

#[test]
fn test_registry_active_plugin() {
    let mut registry = PluginRegistry::new();
    let plugin = Box::new(TestPlugin::new());

    registry.register(plugin).unwrap();
    registry.set_active("test_plugin").unwrap();

    assert_eq!(registry.get_active_id(), Some("test_plugin"));
    assert!(registry.get_active().is_some());
}

#[test]
fn test_registry_register_and_activate() {
    let mut registry = PluginRegistry::new();
    let plugin = Box::new(TestPlugin::new());
    let config = PluginConfig::new(54321);

    let result = registry.register_and_activate(plugin, config);
    assert!(result.is_ok());
    assert_eq!(registry.get_active_id(), Some("test_plugin"));

    // Verify plugin was initialized
    let active = registry.get_active().unwrap();
    let plugin_lock = active.read().unwrap();
    assert_eq!(plugin_lock.get_world_state().seed, 54321);
}

#[test]
fn test_material_tool_requirements() {
    let stone = Material::new("stone".to_string(), "Stone".to_string())
        .with_tool_requirement(ToolType::Pickaxe, ToolTier::Wooden);

    assert!(stone.can_harvest_with(ToolType::Pickaxe, ToolTier::Wooden));
    assert!(stone.can_harvest_with(ToolType::Pickaxe, ToolTier::Iron));
    assert!(!stone.can_harvest_with(ToolType::Axe, ToolTier::Iron));
    assert!(!stone.can_harvest_with(ToolType::Pickaxe, ToolTier::None));
}

#[test]
fn test_crafting_requirements() {
    let recipe = CraftingTemplate::new("pickaxe".to_string(), "Pickaxe".to_string())
        .with_input(Ingredient::new("wood".to_string(), 3))
        .with_input(Ingredient::new("sticks".to_string(), 2));

    let mut inventory = HashMap::new();
    inventory.insert("wood".to_string(), 3);
    inventory.insert("sticks".to_string(), 2);

    assert!(recipe.has_materials(&inventory));

    inventory.insert("wood".to_string(), 2);
    assert!(!recipe.has_materials(&inventory));
}

#[test]
fn test_recipe_book_discovery() {
    let mut book = RecipeBook::new();

    let recipe = CraftingTemplate::new("advanced_item".to_string(), "Advanced Item".to_string())
        .discoverable();

    book.add_recipe(recipe);
    assert!(!book.is_discovered("advanced_item"));

    book.discover_recipe("advanced_item");
    assert!(book.is_discovered("advanced_item"));
}

#[test]
fn test_position_operations() {
    let pos1 = Position::new(0, 0, 0);
    let pos2 = Position::new(3, 4, 0);

    assert_eq!(pos1.distance_to(&pos2), 5.0);

    let pos3: Position = (10, 20, 30).into();
    assert_eq!(pos3.x, 10);
    assert_eq!(pos3.y, 20);
    assert_eq!(pos3.z, 30);
}

#[test]
fn test_action_result_builder() {
    let result = ActionResult::success()
        .with_drive_change(DriveType::Hunger, -0.5)
        .with_item_gained(ItemStack::new("apple".to_string(), 1))
        .with_experience(10.0)
        .with_energy_cost(5.0)
        .with_message("Ate an apple".to_string());

    assert!(result.success);
    assert_eq!(result.drive_changes.get(&DriveType::Hunger), Some(&-0.5));
    assert_eq!(result.items_gained.len(), 1);
    assert_eq!(result.experience, 10.0);
    assert_eq!(result.energy_cost, 5.0);
}

#[test]
fn test_plugin_downcasting() {
    let plugin = TestPlugin::new();

    // Test downcasting
    let any_ref = plugin.as_any();
    let downcast = any_ref.downcast_ref::<TestPlugin>();
    assert!(downcast.is_some());
}
