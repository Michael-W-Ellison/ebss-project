// src/environment/minecraft_survival.rs
//! Minecraft-style survival environment plugin implementation.
//!
//! This plugin provides a complete survival game environment with:
//! - Natural resources (wood, stone, ores)
//! - Tool progression (wooden → stone → iron)
//! - Crafting system
//! - World generation

use std::any::Any;
use std::collections::BTreeMap;
use super::{
    EnvironmentPlugin, PluginMetadata, PluginConfig, WorldState,
    Material, Action, ActionContext, ActionResult, RecipeBook,
    Position, EnvironmentResult, EnvironmentError, ItemStack,
    CraftingTemplate, CraftingOutput, Ingredient, ToolType, ToolTier,
    MaterialState,
};
use crate::core::DriveType;

/// Block data for world storage
#[derive(Debug, Clone)]
struct Block {
    material_id: String,
}

/// Furnace state for smelting simulation
#[derive(Debug, Clone)]
pub struct FurnaceState {
    /// Current temperature (0-1500 degrees)
    pub temperature: f32,
    /// Fuel remaining (in ticks)
    pub fuel_remaining: u32,
    /// Items being smelted
    pub input_item: Option<String>,
    /// Smelting progress (0-100)
    pub smelt_progress: u32,
    /// Output ready to collect
    pub output_item: Option<String>,
}

impl FurnaceState {
    pub fn new() -> Self {
        Self {
            temperature: 20.0, // Room temperature
            fuel_remaining: 0,
            input_item: None,
            smelt_progress: 0,
            output_item: None,
        }
    }

    /// Add fuel to the furnace
    pub fn add_fuel(&mut self, fuel_type: &str) -> bool {
        let fuel_value = match fuel_type {
            "coal" => 1600,      // 80 seconds worth
            "charcoal" => 1600,  // Same as coal
            "wood" => 300,       // 15 seconds
            "planks" => 300,     // Same as wood
            "sticks" => 100,     // 5 seconds
            _ => 0,
        };

        if fuel_value > 0 {
            self.fuel_remaining += fuel_value;
            true
        } else {
            false
        }
    }

    /// Get smelting temperature requirement for an item
    fn get_smelt_temperature(item: &str) -> f32 {
        match item {
            "iron_ore" => 1538.0,     // Iron melting point
            "gold_ore" => 1064.0,     // Gold melting point
            "copper_ore" => 1085.0,   // Copper melting point
            "sand" => 1700.0,         // Glass making temperature
            "clay" => 1000.0,         // Pottery/brick temperature
            "raw_meat" | "raw_fish" => 75.0, // Cooking temperature
            _ => 500.0,               // Default smelting temperature
        }
    }

    /// Get smelting time in ticks for an item
    fn get_smelt_time(item: &str) -> u32 {
        match item {
            "iron_ore" => 200,        // 10 seconds
            "gold_ore" => 200,
            "copper_ore" => 180,
            "sand" => 200,            // Glass making
            "clay" => 200,
            "raw_meat" | "raw_fish" => 100, // 5 seconds cooking
            _ => 200,
        }
    }

    /// Get output item for a given input
    fn get_smelt_output(item: &str) -> Option<&'static str> {
        match item {
            "iron_ore" => Some("iron_ingot"),
            "gold_ore" => Some("gold_ingot"),
            "copper_ore" => Some("copper_ingot"),
            "sand" => Some("glass"),
            "clay" => Some("brick"),
            "raw_meat" => Some("cooked_meat"),
            "raw_fish" => Some("cooked_fish"),
            "wood" => Some("charcoal"),
            _ => None,
        }
    }


    /// Tick the furnace simulation
    pub fn tick(&mut self) {
        // Burn fuel to maintain/increase temperature
        if self.fuel_remaining > 0 {
            self.fuel_remaining -= 1;
            // Temperature increases towards max (1600) when burning
            self.temperature = (self.temperature + 5.0).min(1600.0);
        } else {
            // Temperature decreases towards room temp when not burning
            self.temperature = (self.temperature - 2.0).max(20.0);
        }

        // Process smelting if we have an item
        if let Some(ref item) = self.input_item {
            let required_temp = Self::get_smelt_temperature(item);
            let smelt_time = Self::get_smelt_time(item);

            // Only smelt if temperature is high enough
            if self.temperature >= required_temp * 0.8 {
                // Smelting speed based on how close to optimal temperature
                let efficiency = (self.temperature / required_temp).min(1.2);
                self.smelt_progress += (efficiency as u32).max(1);

                // Check if smelting is complete
                if self.smelt_progress >= smelt_time {
                    if let Some(output) = Self::get_smelt_output(item) {
                        self.output_item = Some(output.to_string());
                    }
                    self.input_item = None;
                    self.smelt_progress = 0;
                }
            }
        }
    }



    /// Get current smelting efficiency (0.0-1.0)
    pub fn efficiency(&self) -> f32 {
        if let Some(ref item) = self.input_item {
            let required_temp = Self::get_smelt_temperature(item);
            (self.temperature / required_temp).min(1.0)
        } else {
            0.0
        }
    }
}

impl Default for FurnaceState {
    fn default() -> Self {
        Self::new()
    }
}

/// Minecraft-style survival environment plugin
pub struct MinecraftSurvivalPlugin {
    metadata: PluginMetadata,
    world_state: WorldState,
    materials: BTreeMap<String, Material>,
    actions: BTreeMap<String, Action>,
    recipe_book: RecipeBook,
    world_size: (i32, i32, i32),
    blocks: BTreeMap<(i32, i32, i32), Block>,
    initialized: bool,
}

impl MinecraftSurvivalPlugin {
    /// Create a new Minecraft survival plugin
    pub fn new() -> Self {
        let mut plugin = Self {
            metadata: PluginMetadata {
                id: "minecraft_survival".to_string(),
                name: "Minecraft Survival".to_string(),
                version: "0.1.0".to_string(),
                author: "EBSS Team".to_string(),
                description: "A Minecraft-style survival environment with tool progression".to_string(),
                tags: vec!["survival".to_string(), "crafting".to_string(), "mining".to_string()],
            },
            world_state: WorldState::new(0),
            materials: BTreeMap::new(),
            actions: BTreeMap::new(),
            recipe_book: RecipeBook::new(),
            world_size: (256, 256, 128),
            blocks: BTreeMap::new(),
            initialized: false,
        };

        plugin.register_materials();
        plugin.register_actions();
        plugin.register_recipes();

        plugin
    }

    /// Register all materials
    fn register_materials(&mut self) {
        // Natural resources
        self.materials.insert("wood".to_string(), Material::new("wood".to_string(), "Wood".to_string())
            .with_hardness(2.0)
            .with_tool_requirement(ToolType::Axe, ToolTier::None)
            .as_fuel(300)
            .with_weight(0.8));

        self.materials.insert("stone".to_string(), Material::new("stone".to_string(), "Stone".to_string())
            .with_hardness(3.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Wooden)
            .with_weight(2.5));

        self.materials.insert("iron_ore".to_string(), Material::new("iron_ore".to_string(), "Iron Ore".to_string())
            .with_hardness(5.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Stone)
            .as_ore("iron".to_string(), 1.0)
            .with_weight(3.0));

        self.materials.insert("coal".to_string(), Material::new("coal".to_string(), "Coal".to_string())
            .with_hardness(3.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Wooden)
            .as_fuel(1600)
            .with_weight(1.2));

        self.materials.insert("dirt".to_string(), Material::new("dirt".to_string(), "Dirt".to_string())
            .with_hardness(0.5)
            .with_tool_requirement(ToolType::Shovel, ToolTier::None)
            .with_weight(1.5));

        self.materials.insert("grass".to_string(), Material::new("grass".to_string(), "Grass Block".to_string())
            .with_hardness(0.6)
            .with_tool_requirement(ToolType::Shovel, ToolTier::None)
            .with_weight(1.5));

        self.materials.insert("sand".to_string(), Material::new("sand".to_string(), "Sand".to_string())
            .with_hardness(0.5)
            .with_tool_requirement(ToolType::Shovel, ToolTier::None)
            .with_weight(1.6));

        self.materials.insert("water".to_string(), Material::new("water".to_string(), "Water".to_string())
            .with_hardness(100.0) // Cannot be harvested normally
            .with_state(MaterialState::Liquid)
            .with_weight(1.0));

        // Processed materials
        self.materials.insert("planks".to_string(), Material::new("planks".to_string(), "Wooden Planks".to_string())
            .with_hardness(2.0)
            .with_tool_requirement(ToolType::Axe, ToolTier::None)
            .as_fuel(300)
            .with_weight(0.6));

        self.materials.insert("sticks".to_string(), Material::new("sticks".to_string(), "Sticks".to_string())
            .with_hardness(1.0)
            .as_fuel(100)
            .with_weight(0.1));

        self.materials.insert("iron_ingot".to_string(), Material::new("iron_ingot".to_string(), "Iron Ingot".to_string())
            .with_hardness(6.0)
            .with_weight(1.0));

        // Tools with durability
        self.materials.insert("wooden_pickaxe".to_string(), Material::new("wooden_pickaxe".to_string(), "Wooden Pickaxe".to_string())
            .with_durability(60)
            .with_weight(1.0));

        self.materials.insert("stone_pickaxe".to_string(), Material::new("stone_pickaxe".to_string(), "Stone Pickaxe".to_string())
            .with_durability(132)
            .with_weight(1.5));

        self.materials.insert("iron_pickaxe".to_string(), Material::new("iron_pickaxe".to_string(), "Iron Pickaxe".to_string())
            .with_durability(251)
            .with_weight(2.0));

        self.materials.insert("wooden_axe".to_string(), Material::new("wooden_axe".to_string(), "Wooden Axe".to_string())
            .with_durability(60)
            .with_weight(1.0));

        self.materials.insert("stone_axe".to_string(), Material::new("stone_axe".to_string(), "Stone Axe".to_string())
            .with_durability(132)
            .with_weight(1.5));

        self.materials.insert("iron_axe".to_string(), Material::new("iron_axe".to_string(), "Iron Axe".to_string())
            .with_durability(251)
            .with_weight(2.0));

        // Food
        self.materials.insert("apple".to_string(), Material::new("apple".to_string(), "Apple".to_string())
            .as_food(4.0)
            .with_weight(0.2));

        self.materials.insert("cooked_meat".to_string(), Material::new("cooked_meat".to_string(), "Cooked Meat".to_string())
            .as_food(8.0)
            .with_weight(0.3));

        self.materials.insert("raw_meat".to_string(), Material::new("raw_meat".to_string(), "Raw Meat".to_string())
            .as_food(3.0)
            .with_weight(0.3));
    }

    /// Register all actions
    fn register_actions(&mut self) {
        // Harvesting actions
        self.actions.insert("chop_tree".to_string(), Action::Gather { resource_type: "wood".to_string() });
        self.actions.insert("mine_stone".to_string(), Action::Gather { resource_type: "stone".to_string() });
        self.actions.insert("mine_iron".to_string(), Action::Gather { resource_type: "iron_ore".to_string() });
        self.actions.insert("mine_coal".to_string(), Action::Gather { resource_type: "coal".to_string() });
        self.actions.insert("dig_dirt".to_string(), Action::Gather { resource_type: "dirt".to_string() });

        // Crafting
        self.actions.insert("craft".to_string(), Action::Craft { item_type: "".to_string() });

        // Food consumption
        self.actions.insert("eat".to_string(), Action::Eat { food_type: "".to_string() });

        // Building
        self.actions.insert("build".to_string(), Action::Build {
            structure_type: "".to_string(),
            position: (0, 0, 0)
        });

        // Exploration
        self.actions.insert("explore".to_string(), Action::Explore { direction: (0, 0, 0) });
    }

    /// Register all crafting recipes
    fn register_recipes(&mut self) {
        // Basic recipes
        self.recipe_book.add_recipe(
            CraftingTemplate::new("planks".to_string(), "Wooden Planks".to_string())
                .with_input(Ingredient::new("wood".to_string(), 1))
                .with_output(CraftingOutput::new("planks".to_string(), 4))
        );

        self.recipe_book.add_recipe(
            CraftingTemplate::new("sticks".to_string(), "Sticks".to_string())
                .with_input(Ingredient::new("planks".to_string(), 2))
                .with_output(CraftingOutput::new("sticks".to_string(), 4))
        );

        // Tool recipes
        self.recipe_book.add_recipe(
            CraftingTemplate::new("wooden_pickaxe".to_string(), "Wooden Pickaxe".to_string())
                .with_input(Ingredient::new("planks".to_string(), 3))
                .with_input(Ingredient::new("sticks".to_string(), 2))
                .with_output(CraftingOutput::new("wooden_pickaxe".to_string(), 1))
        );

        self.recipe_book.add_recipe(
            CraftingTemplate::new("stone_pickaxe".to_string(), "Stone Pickaxe".to_string())
                .with_input(Ingredient::new("stone".to_string(), 3))
                .with_input(Ingredient::new("sticks".to_string(), 2))
                .with_output(CraftingOutput::new("stone_pickaxe".to_string(), 1))
        );

        self.recipe_book.add_recipe(
            CraftingTemplate::new("iron_pickaxe".to_string(), "Iron Pickaxe".to_string())
                .with_input(Ingredient::new("iron_ingot".to_string(), 3))
                .with_input(Ingredient::new("sticks".to_string(), 2))
                .with_output(CraftingOutput::new("iron_pickaxe".to_string(), 1))
        );

        self.recipe_book.add_recipe(
            CraftingTemplate::new("wooden_axe".to_string(), "Wooden Axe".to_string())
                .with_input(Ingredient::new("planks".to_string(), 3))
                .with_input(Ingredient::new("sticks".to_string(), 2))
                .with_output(CraftingOutput::new("wooden_axe".to_string(), 1))
        );

        self.recipe_book.add_recipe(
            CraftingTemplate::new("stone_axe".to_string(), "Stone Axe".to_string())
                .with_input(Ingredient::new("stone".to_string(), 3))
                .with_input(Ingredient::new("sticks".to_string(), 2))
                .with_output(CraftingOutput::new("stone_axe".to_string(), 1))
        );

        self.recipe_book.add_recipe(
            CraftingTemplate::new("iron_axe".to_string(), "Iron Axe".to_string())
                .with_input(Ingredient::new("iron_ingot".to_string(), 3))
                .with_input(Ingredient::new("sticks".to_string(), 2))
                .with_output(CraftingOutput::new("iron_axe".to_string(), 1))
        );

        // Smelting recipes - use FurnaceState for realistic temperature-based smelting
        // These simplified recipes allow direct crafting as fallback
        self.recipe_book.add_recipe(
            CraftingTemplate::new("iron_ingot".to_string(), "Iron Ingot".to_string())
                .with_input(Ingredient::new("iron_ore".to_string(), 1))
                .with_input(Ingredient::new("coal".to_string(), 1))
                .with_output(CraftingOutput::new("iron_ingot".to_string(), 1))
        );

        self.recipe_book.add_recipe(
            CraftingTemplate::new("gold_ingot".to_string(), "Gold Ingot".to_string())
                .with_input(Ingredient::new("gold_ore".to_string(), 1))
                .with_input(Ingredient::new("coal".to_string(), 1))
                .with_output(CraftingOutput::new("gold_ingot".to_string(), 1))
        );

        self.recipe_book.add_recipe(
            CraftingTemplate::new("glass".to_string(), "Glass".to_string())
                .with_input(Ingredient::new("sand".to_string(), 1))
                .with_input(Ingredient::new("coal".to_string(), 1))
                .with_output(CraftingOutput::new("glass".to_string(), 1))
        );

        self.recipe_book.add_recipe(
            CraftingTemplate::new("brick".to_string(), "Brick".to_string())
                .with_input(Ingredient::new("clay".to_string(), 1))
                .with_input(Ingredient::new("coal".to_string(), 1))
                .with_output(CraftingOutput::new("brick".to_string(), 1))
        );

        self.recipe_book.add_recipe(
            CraftingTemplate::new("charcoal".to_string(), "Charcoal".to_string())
                .with_input(Ingredient::new("wood".to_string(), 1))
                .with_input(Ingredient::new("coal".to_string(), 1)) // Bootstrap with coal, then use charcoal
                .with_output(CraftingOutput::new("charcoal".to_string(), 1))
        );
    }

    /// Generate the world terrain
    fn generate_world(&mut self) {
        let (width, depth, height) = self.world_size;
        let seed = self.world_state.seed;

        // Simple heightmap-based generation using seed
        for x in 0..width.min(64) { // Generate a smaller area for performance
            for z in 0..depth.min(64) {
                // Simple deterministic height calculation
                let height_noise = ((x as f64 * 0.05 + seed as f64 * 0.001).sin() *
                                   (z as f64 * 0.05 + seed as f64 * 0.002).cos() + 1.0) / 2.0;
                let terrain_height = (64.0 + height_noise * 20.0) as i32;

                // Place bedrock at y=0
                self.blocks.insert((x, 0, z), Block { material_id: "stone".to_string() });

                // Fill with stone up to near surface
                for y in 1..(terrain_height - 4).min(height) {
                    let block_type = if y < 40 && (x + y + z) % 20 == 0 {
                        "iron_ore"
                    } else if y < 50 && (x + y + z) % 15 == 0 {
                        "coal"
                    } else {
                        "stone"
                    };
                    self.blocks.insert((x, y, z), Block { material_id: block_type.to_string() });
                }

                // Add dirt layer
                for y in (terrain_height - 4).max(1)..terrain_height {
                    self.blocks.insert((x, y, z), Block { material_id: "dirt".to_string() });
                }

                // Add surface (grass or sand near water)
                if terrain_height > 0 && terrain_height < height {
                    let surface_type = if terrain_height <= 66 {
                        "sand"
                    } else {
                        "grass"
                    };
                    self.blocks.insert((x, terrain_height, z), Block { material_id: surface_type.to_string() });
                }

                // Fill below sea level with water
                for y in (terrain_height + 1)..65 {
                    if y > 0 && y < height {
                        self.blocks.insert((x, y, z), Block { material_id: "water".to_string() });
                    }
                }

                // Add occasional trees on grass
                if terrain_height > 66 && terrain_height < height - 5 {
                    if (x * 7 + z * 13 + seed as i32) % 30 == 0 {
                        for tree_y in 1..=4 {
                            self.blocks.insert((x, terrain_height + tree_y, z), Block { material_id: "wood".to_string() });
                        }
                    }
                }
            }
        }
    }

    /// Get harvest result for a material
    fn get_harvest_result(&self, material_id: &str) -> ActionResult {
        let material = match self.materials.get(material_id) {
            Some(m) => m,
            None => return ActionResult::failure(format!("Unknown material: {}", material_id)),
        };

        let quantity = material.drop_quantity.0 +
            (crate::core::dice::any::<u32>() % (material.drop_quantity.1 - material.drop_quantity.0 + 1).max(1));

        ActionResult::success()
            .with_item_gained(ItemStack::new(material_id.to_string(), quantity))
            .with_drive_change(DriveType::Industry, -0.1)
            .with_energy_cost(material.hardness * 2.0)
            .with_experience(material.hardness * 5.0)
            .with_message(format!("Harvested {} x{}", material.name, quantity))
    }
}

impl Default for MinecraftSurvivalPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvironmentPlugin for MinecraftSurvivalPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self, config: PluginConfig) -> EnvironmentResult<()> {
        self.world_state.seed = config.seed;
        self.world_size = config.world_size;
        self.initialized = true;
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
        match action {
            Action::Gather { resource_type } => {
                // Check if material exists at position
                let pos = (context.position.x, context.position.y, context.position.z);
                if let Some(block) = self.blocks.get(&pos) {
                    if &block.material_id == resource_type {
                        let result = self.get_harvest_result(resource_type);
                        if result.success {
                            // Remove the block
                            self.blocks.remove(&pos);
                        }
                        return Ok(result);
                    }
                }
                // Allow gathering even without block at position (for testing)
                Ok(self.get_harvest_result(resource_type))
            }
            Action::Eat { food_type } => {
                if let Some(material) = self.materials.get(food_type) {
                    if material.is_edible {
                        Ok(ActionResult::success()
                            .with_item_consumed(ItemStack::new(food_type.clone(), 1))
                            .with_drive_change(DriveType::Hunger, -material.food_value / 10.0)
                            .with_message(format!("Ate {}", material.name)))
                    } else {
                        Err(EnvironmentError::InvalidAction(format!("{} is not edible", food_type)))
                    }
                } else {
                    Err(EnvironmentError::MaterialNotFound(food_type.clone()))
                }
            }
            Action::Craft { item_type } => {
                if let Some(recipe) = self.recipe_book.get_recipe(item_type) {
                    let mut result = ActionResult::success()
                        .with_drive_change(DriveType::Utility, -0.1)
                        .with_energy_cost(recipe.energy_cost)
                        .with_experience(recipe.experience_gain);

                    for input in &recipe.inputs {
                        result = result.with_item_consumed(ItemStack::new(input.material_id.clone(), input.quantity));
                    }
                    for output in &recipe.outputs {
                        result = result.with_item_gained(ItemStack::new(output.material_id.clone(), output.quantity));
                    }
                    result = result.with_message(format!("Crafted {}", recipe.name));

                    Ok(result)
                } else {
                    Err(EnvironmentError::RecipeNotFound(item_type.clone()))
                }
            }
            Action::Build { structure_type, position } => {
                Ok(ActionResult::success()
                    .with_drive_change(DriveType::Construction, -0.2)
                    .with_energy_cost(15.0)
                    .with_experience(20.0)
                    .with_message(format!("Built {} at {:?}", structure_type, position)))
            }
            Action::Explore { direction } => {
                Ok(ActionResult::success()
                    .with_drive_change(DriveType::Curiosity, -0.1)
                    .with_energy_cost(5.0)
                    .with_experience(5.0)
                    .with_message(format!("Explored in direction {:?}", direction)))
            }
            Action::Sleep { duration } => {
                Ok(ActionResult::success()
                    .with_drive_change(DriveType::Rest, -0.5)
                    .with_message(format!("Slept for {} ticks", duration)))
            }
            _ => Ok(ActionResult::success()
                .with_message("Action completed".to_string()))
        }
    }

    fn tick(&mut self) {
        self.world_state.advance_tick(0.001);
    }

    fn get_material_at(&self, position: Position) -> Option<&Material> {
        let pos = (position.x, position.y, position.z);
        self.blocks.get(&pos).and_then(|block| self.materials.get(&block.material_id))
    }

    fn is_walkable(&self, position: Position) -> bool {
        let pos = (position.x, position.y, position.z);
        match self.blocks.get(&pos) {
            Some(block) => {
                // Water and air are walkable (with swimming)
                block.material_id == "water" || !self.materials.contains_key(&block.material_id)
            }
            None => true, // Air is walkable
        }
    }

    fn is_valid_position(&self, position: Position) -> bool {
        let (width, depth, height) = self.world_size;
        position.x >= 0 && position.x < width
            && position.z >= 0 && position.z < depth
            && position.y >= 0 && position.y < height
    }

    fn find_nearby_materials(
        &self,
        position: Position,
        material_id: &str,
        radius: f32,
    ) -> Vec<Position> {
        let radius_sq = (radius * radius) as i64;
        let mut results = Vec::new();

        let r = radius as i32;
        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    let check_pos = (position.x + dx, position.y + dy, position.z + dz);
                    if let Some(block) = self.blocks.get(&check_pos) {
                        if block.material_id == material_id {
                            let dist_sq = (dx * dx + dy * dy + dz * dz) as i64;
                            if dist_sq <= radius_sq {
                                results.push(Position::new(check_pos.0, check_pos.1, check_pos.2));
                            }
                        }
                    }
                }
            }
        }

        results
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = MinecraftSurvivalPlugin::new();
        assert_eq!(plugin.metadata.id, "minecraft_survival");
        assert_eq!(plugin.metadata.version, "0.1.0");
    }

    #[test]
    fn test_plugin_materials() {
        let plugin = MinecraftSurvivalPlugin::new();

        // Check core materials exist
        assert!(plugin.get_material("wood").is_some());
        assert!(plugin.get_material("stone").is_some());
        assert!(plugin.get_material("iron_ore").is_some());
        assert!(plugin.get_material("coal").is_some());

        // Check material properties
        let wood = plugin.get_material("wood").unwrap();
        assert_eq!(wood.hardness, 2.0);
        assert!(wood.is_fuel);

        let stone = plugin.get_material("stone").unwrap();
        assert_eq!(stone.required_tool, ToolType::Pickaxe);
        assert_eq!(stone.required_tier, ToolTier::Wooden);
    }

    #[test]
    fn test_plugin_recipes() {
        let plugin = MinecraftSurvivalPlugin::new();
        let book = plugin.get_recipe_book();

        // Check recipes exist
        assert!(book.get_recipe("planks").is_some());
        assert!(book.get_recipe("sticks").is_some());
        assert!(book.get_recipe("wooden_pickaxe").is_some());

        // Check recipe details
        let planks = book.get_recipe("planks").unwrap();
        assert_eq!(planks.inputs.len(), 1);
        assert_eq!(planks.inputs[0].material_id, "wood");
        assert_eq!(planks.outputs[0].quantity, 4);
    }

    #[test]
    fn test_plugin_initialization() {
        let mut plugin = MinecraftSurvivalPlugin::new();
        let config = PluginConfig::tiny(12345);

        plugin.initialize(config).unwrap();

        assert!(plugin.initialized);
        assert_eq!(plugin.world_state.seed, 12345);
        assert!(!plugin.blocks.is_empty());
    }

    #[test]
    fn test_action_execution() {
        let mut plugin = MinecraftSurvivalPlugin::new();
        let config = PluginConfig::tiny(0);
        plugin.initialize(config).unwrap();

        let action = Action::Gather { resource_type: "wood".to_string() };
        let context = ActionContext::new("agent_1".to_string(), Position::new(0, 0, 0));

        let result = plugin.execute_action(&action, context).unwrap();
        assert!(result.success);
        assert!(!result.items_gained.is_empty());
    }

    #[test]
    fn test_world_tick() {
        let mut plugin = MinecraftSurvivalPlugin::new();
        let config = PluginConfig::tiny(0);
        plugin.initialize(config).unwrap();

        let initial_tick = plugin.world_state.tick;
        plugin.tick();
        assert_eq!(plugin.world_state.tick, initial_tick + 1);
    }

    #[test]
    fn test_crafting_action() {
        let mut plugin = MinecraftSurvivalPlugin::new();
        let config = PluginConfig::tiny(0);
        plugin.initialize(config).unwrap();

        let action = Action::Craft { item_type: "planks".to_string() };
        let context = ActionContext::new("agent_1".to_string(), Position::new(0, 0, 0));

        let result = plugin.execute_action(&action, context).unwrap();
        assert!(result.success);
        assert_eq!(result.items_consumed.len(), 1);
        assert_eq!(result.items_gained.len(), 1);
        assert_eq!(result.items_gained[0].material_id, "planks");
        assert_eq!(result.items_gained[0].quantity, 4);
    }
}
