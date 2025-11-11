// plugins/minecraft_survival/src/lib.rs
//! Minecraft-style survival environment plugin for EBSS.
//!
//! This plugin provides a Minecraft-inspired survival environment with:
//! - Basic materials (wood, stone, iron, etc.)
//! - Tool progression system
//! - Crafting recipes
//! - Basic world generation

use ebss::environment::*;
use ebss::core::DriveType;
use std::any::Any;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rand::Rng;

/// Minecraft-style survival environment plugin
pub struct MinecraftSurvivalPlugin {
    metadata: PluginMetadata,
    world_state: WorldState,
    materials: HashMap<String, Material>,
    actions: HashMap<String, Action>,
    recipe_book: RecipeBook,
    world_map: HashMap<Position, String>, // Position -> Material ID
    config: Option<PluginConfig>,
}

impl MinecraftSurvivalPlugin {
    pub fn new() -> Self {
        let metadata = PluginMetadata {
            id: "minecraft_survival".to_string(),
            name: "Minecraft Survival".to_string(),
            version: "0.1.0".to_string(),
            author: "EBSS Team".to_string(),
            description: "A Minecraft-inspired survival environment with crafting, resource gathering, and tool progression.".to_string(),
            tags: vec![
                "survival".to_string(),
                "crafting".to_string(),
                "minecraft".to_string(),
            ],
        };

        let mut plugin = Self {
            metadata,
            world_state: WorldState::new(0),
            materials: HashMap::new(),
            actions: HashMap::new(),
            recipe_book: RecipeBook::new(),
            world_map: HashMap::new(),
            config: None,
        };

        plugin.register_materials();
        plugin.register_actions();
        plugin.register_recipes();

        plugin
    }

    fn register_materials(&mut self) {
        // Natural resources
        let wood = Material::new("wood".to_string(), "Wood".to_string())
            .with_description("Raw wood from trees".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(2.0)
            .with_tool_requirement(ToolType::Axe, ToolTier::None)
            .with_harvest_time(100)
            .with_drop_quantity(1, 3)
            .as_fuel(300)
            .flammable();

        let stone = Material::new("stone".to_string(), "Stone".to_string())
            .with_description("Basic stone material".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(3.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Wooden)
            .with_harvest_time(150)
            .with_drop_quantity(1, 1);

        let iron_ore = Material::new("iron_ore".to_string(), "Iron Ore".to_string())
            .with_description("Raw iron ore that needs smelting".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(5.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Stone)
            .with_harvest_time(200)
            .with_drop_quantity(1, 1);

        let coal = Material::new("coal".to_string(), "Coal".to_string())
            .with_description("Fuel source and crafting material".to_string())
            .with_category(MaterialCategory::Fuel)
            .with_hardness(3.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Wooden)
            .with_harvest_time(100)
            .with_drop_quantity(1, 1)
            .as_fuel(1600);

        // Processed materials
        let planks = Material::new("planks".to_string(), "Wooden Planks".to_string())
            .with_description("Processed wood for building".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(1.5)
            .as_fuel(200)
            .flammable();

        let sticks = Material::new("sticks".to_string(), "Sticks".to_string())
            .with_description("Basic crafting component".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(0.5)
            .as_fuel(100);

        let iron_ingot = Material::new("iron_ingot".to_string(), "Iron Ingot".to_string())
            .with_description("Smelted iron for tools and equipment".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(4.0);

        // Tools
        let wooden_pickaxe = Material::new("wooden_pickaxe".to_string(), "Wooden Pickaxe".to_string())
            .with_description("Basic mining tool".to_string())
            .with_category(MaterialCategory::Tool)
            .with_durability(60)
            .with_stack_size(1)
            .as_fuel(200);

        let stone_pickaxe = Material::new("stone_pickaxe".to_string(), "Stone Pickaxe".to_string())
            .with_description("Improved mining tool".to_string())
            .with_category(MaterialCategory::Tool)
            .with_durability(132)
            .with_stack_size(1);

        let iron_pickaxe = Material::new("iron_pickaxe".to_string(), "Iron Pickaxe".to_string())
            .with_description("Advanced mining tool".to_string())
            .with_category(MaterialCategory::Tool)
            .with_durability(251)
            .with_stack_size(1);

        let wooden_axe = Material::new("wooden_axe".to_string(), "Wooden Axe".to_string())
            .with_description("Basic woodcutting tool".to_string())
            .with_category(MaterialCategory::Tool)
            .with_durability(60)
            .with_stack_size(1)
            .as_fuel(200);

        // Food
        let apple = Material::new("apple".to_string(), "Apple".to_string())
            .with_description("Restores hunger".to_string())
            .with_category(MaterialCategory::Food)
            .as_food(4.0)
            .with_stack_size(16);

        // Register all materials
        for material in vec![
            wood, stone, iron_ore, coal, planks, sticks, iron_ingot,
            wooden_pickaxe, stone_pickaxe, iron_pickaxe, wooden_axe, apple,
        ] {
            self.materials.insert(material.id.clone(), material);
        }
    }

    fn register_actions(&mut self) {
        // Harvest wood
        let chop_tree = Action::new(
            "chop_tree".to_string(),
            "Chop Tree".to_string(),
            ActionType::Harvest,
        )
        .with_description("Chop down a tree for wood".to_string())
        .with_effects(
            ActionEffects::none()
                .with_energy_cost(5.0)
                .with_time_cost(100)
                .with_drive_effect(DriveType::Industry, -0.1)
                .with_experience("woodcutting".to_string(), 10.0),
        );

        // Mine stone
        let mine_stone = Action::new(
            "mine_stone".to_string(),
            "Mine Stone".to_string(),
            ActionType::Harvest,
        )
        .with_description("Mine stone with a pickaxe".to_string())
        .with_requirements(
            ActionRequirements::none()
                .with_tool(ToolType::Pickaxe, ToolTier::Wooden),
        )
        .with_effects(
            ActionEffects::none()
                .with_energy_cost(8.0)
                .with_time_cost(150)
                .with_drive_effect(DriveType::Industry, -0.15)
                .with_experience("mining".to_string(), 15.0),
        );

        // Craft
        let craft = Action::new(
            "craft".to_string(),
            "Craft".to_string(),
            ActionType::Craft,
        )
        .with_description("Craft an item from materials".to_string())
        .with_effects(
            ActionEffects::none()
                .with_energy_cost(2.0)
                .with_time_cost(20)
                .with_drive_effect(DriveType::Utility, -0.2)
                .with_experience("crafting".to_string(), 5.0),
        );

        // Eat food
        let eat = Action::new(
            "eat".to_string(),
            "Eat".to_string(),
            ActionType::Eat,
        )
        .with_description("Consume food to restore hunger".to_string())
        .with_effects(
            ActionEffects::none()
                .with_time_cost(10)
                .with_drive_effect(DriveType::Hunger, -0.5),
        );

        // Register all actions
        for action in vec![chop_tree, mine_stone, craft, eat] {
            self.actions.insert(action.id.clone(), action);
        }
    }

    fn register_recipes(&mut self) {
        // Wood -> Planks
        let planks_recipe = CraftingTemplate::new(
            "planks".to_string(),
            "Wooden Planks".to_string(),
        )
        .with_description("Convert wood into planks".to_string())
        .with_input(Ingredient::new("wood".to_string(), 1))
        .with_output(CraftingOutput::new("planks".to_string(), 4))
        .with_craft_time(10)
        .with_energy_cost(1.0)
        .with_experience(2.0);

        // Planks -> Sticks
        let sticks_recipe = CraftingTemplate::new(
            "sticks".to_string(),
            "Sticks".to_string(),
        )
        .with_description("Convert planks into sticks".to_string())
        .with_input(Ingredient::new("planks".to_string(), 2))
        .with_output(CraftingOutput::new("sticks".to_string(), 4))
        .with_craft_time(10)
        .with_energy_cost(1.0)
        .with_experience(2.0);

        // Wooden Pickaxe
        let wooden_pickaxe_recipe = CraftingTemplate::new(
            "wooden_pickaxe".to_string(),
            "Wooden Pickaxe".to_string(),
        )
        .with_description("Craft a basic pickaxe".to_string())
        .with_input(Ingredient::new("planks".to_string(), 3))
        .with_input(Ingredient::new("sticks".to_string(), 2))
        .with_output(CraftingOutput::new("wooden_pickaxe".to_string(), 1))
        .at_station(CraftingStation::Workbench)
        .with_craft_time(20)
        .with_energy_cost(5.0)
        .with_experience(10.0);

        // Stone Pickaxe
        let stone_pickaxe_recipe = CraftingTemplate::new(
            "stone_pickaxe".to_string(),
            "Stone Pickaxe".to_string(),
        )
        .with_description("Craft an improved pickaxe".to_string())
        .with_input(Ingredient::new("stone".to_string(), 3))
        .with_input(Ingredient::new("sticks".to_string(), 2))
        .with_output(CraftingOutput::new("stone_pickaxe".to_string(), 1))
        .at_station(CraftingStation::Workbench)
        .with_craft_time(20)
        .with_energy_cost(5.0)
        .with_experience(15.0);

        // Iron Pickaxe
        let iron_pickaxe_recipe = CraftingTemplate::new(
            "iron_pickaxe".to_string(),
            "Iron Pickaxe".to_string(),
        )
        .with_description("Craft an advanced pickaxe".to_string())
        .with_input(Ingredient::new("iron_ingot".to_string(), 3))
        .with_input(Ingredient::new("sticks".to_string(), 2))
        .with_output(CraftingOutput::new("iron_pickaxe".to_string(), 1))
        .at_station(CraftingStation::Workbench)
        .with_craft_time(30)
        .with_energy_cost(8.0)
        .with_experience(25.0);

        // Wooden Axe
        let wooden_axe_recipe = CraftingTemplate::new(
            "wooden_axe".to_string(),
            "Wooden Axe".to_string(),
        )
        .with_description("Craft a basic axe".to_string())
        .with_input(Ingredient::new("planks".to_string(), 3))
        .with_input(Ingredient::new("sticks".to_string(), 2))
        .with_output(CraftingOutput::new("wooden_axe".to_string(), 1))
        .at_station(CraftingStation::Workbench)
        .with_craft_time(20)
        .with_energy_cost(5.0)
        .with_experience(10.0);

        // Iron Ingot (smelting)
        let iron_ingot_recipe = CraftingTemplate::new(
            "iron_ingot".to_string(),
            "Iron Ingot".to_string(),
        )
        .with_description("Smelt iron ore into ingots".to_string())
        .with_input(Ingredient::new("iron_ore".to_string(), 1))
        .with_input(Ingredient::new("coal".to_string(), 1))
        .with_output(CraftingOutput::new("iron_ingot".to_string(), 1))
        .at_station(CraftingStation::Furnace)
        .with_craft_time(100)
        .with_energy_cost(3.0)
        .with_experience(20.0);

        // Register all recipes
        for recipe in vec![
            planks_recipe,
            sticks_recipe,
            wooden_pickaxe_recipe,
            stone_pickaxe_recipe,
            iron_pickaxe_recipe,
            wooden_axe_recipe,
            iron_ingot_recipe,
        ] {
            self.recipe_book.add_recipe(recipe);
        }
    }

    fn generate_world(&mut self) {
        let config = self.config.as_ref().unwrap();
        let (width, depth, height) = config.world_size;
        let mut rng = rand::thread_rng();

        // Simple world generation: distribute resources
        for x in -width/2..width/2 {
            for z in -depth/2..depth/2 {
                // Surface layer - trees (wood)
                if rng.gen_bool(0.05) {
                    self.world_map.insert(
                        Position::new(x, 64, z),
                        "wood".to_string(),
                    );
                }

                // Stone layer
                for y in 0..64 {
                    if rng.gen_bool(0.3) {
                        self.world_map.insert(
                            Position::new(x, y, z),
                            "stone".to_string(),
                        );
                    }
                }

                // Coal deposits
                if rng.gen_bool(0.02) {
                    let y = rng.gen_range(10..50);
                    self.world_map.insert(
                        Position::new(x, y, z),
                        "coal".to_string(),
                    );
                }

                // Iron ore deposits
                if rng.gen_bool(0.01) {
                    let y = rng.gen_range(5..40);
                    self.world_map.insert(
                        Position::new(x, y, z),
                        "iron_ore".to_string(),
                    );
                }
            }
        }
    }
}

impl EnvironmentPlugin for MinecraftSurvivalPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self, config: PluginConfig) -> EnvironmentResult<()> {
        self.world_state.seed = config.seed;
        self.config = Some(config);
        self.generate_world();
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
        context: ActionContext,
    ) -> EnvironmentResult<ActionResult> {
        // Simple action execution logic
        let mut result = ActionResult::success()
            .with_energy_cost(action.effects.energy_cost);

        // Add drive changes
        for (drive, amount) in &action.effects.drive_effects {
            result = result.with_drive_change(*drive, *amount);
        }

        // Add experience
        for (skill, exp) in &action.effects.experience_gain {
            result.experience += exp;
        }

        // Handle specific action types
        match &action.action_type {
            ActionType::Harvest => {
                if let Some(material_id) = context.target_material {
                    if let Some(material) = self.materials.get(&material_id) {
                        let quantity = rand::thread_rng().gen_range(
                            material.drop_quantity.0..=material.drop_quantity.1
                        );
                        result = result.with_item_gained(ItemStack::new(material_id, quantity));
                    }
                }
            }
            ActionType::Eat => {
                if let Some(material_id) = context.target_material {
                    if let Some(material) = self.materials.get(&material_id) {
                        if material.is_edible {
                            result = result
                                .with_drive_change(DriveType::Hunger, -material.food_value * 0.1)
                                .with_item_consumed(ItemStack::new(material_id, 1));
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(result)
    }

    fn tick(&mut self) {
        self.world_state.advance_tick(0.001);
    }

    fn get_material_at(&self, position: Position) -> Option<&Material> {
        self.world_map
            .get(&position)
            .and_then(|id| self.materials.get(id))
    }

    fn is_walkable(&self, position: Position) -> bool {
        self.world_map.get(&position).is_none()
    }

    fn is_valid_position(&self, position: Position) -> bool {
        if let Some(config) = &self.config {
            let (width, depth, height) = config.world_size;
            position.x >= -width/2 && position.x < width/2
                && position.z >= -depth/2 && position.z < depth/2
                && position.y >= 0 && position.y < height
        } else {
            false
        }
    }

    fn find_nearby_materials(
        &self,
        position: Position,
        material_id: &str,
        radius: f32,
    ) -> Vec<Position> {
        self.world_map
            .iter()
            .filter(|(pos, id)| {
                *id == material_id && position.distance_to(pos) <= radius
            })
            .map(|(pos, _)| *pos)
            .collect()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for MinecraftSurvivalPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = MinecraftSurvivalPlugin::new();
        assert_eq!(plugin.metadata.id, "minecraft_survival");
        assert!(!plugin.materials.is_empty());
        assert!(!plugin.actions.is_empty());
    }

    #[test]
    fn test_plugin_initialization() {
        let mut plugin = MinecraftSurvivalPlugin::new();
        let config = PluginConfig::new(12345);

        let result = plugin.initialize(config);
        assert!(result.is_ok());
        assert_eq!(plugin.world_state.seed, 12345);
    }

    #[test]
    fn test_get_materials() {
        let plugin = MinecraftSurvivalPlugin::new();
        let materials = plugin.get_materials();
        assert!(!materials.is_empty());

        // Check for specific materials
        assert!(plugin.get_material("wood").is_some());
        assert!(plugin.get_material("stone").is_some());
        assert!(plugin.get_material("iron_ore").is_some());
    }

    #[test]
    fn test_recipe_book() {
        let plugin = MinecraftSurvivalPlugin::new();
        let book = plugin.get_recipe_book();

        assert!(book.get_recipe("planks").is_some());
        assert!(book.get_recipe("wooden_pickaxe").is_some());
    }
}
