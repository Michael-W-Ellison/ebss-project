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
use noise::{NoiseFn, Perlin, Seedable};

/// Minecraft-style survival environment plugin
pub struct MinecraftSurvivalPlugin {
    metadata: PluginMetadata,
    world_state: WorldState,
    materials: HashMap<String, Material>,
    actions: HashMap<String, Action>,
    recipe_book: RecipeBook,
    world_map: HashMap<Position, String>, // Position -> Material ID
    structures: StructureRegistry,
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
            structures: StructureRegistry::new(),
            config: None,
        };

        plugin.register_materials();
        plugin.register_actions();
        plugin.register_recipes();

        plugin
    }

    fn register_materials(&mut self) {
        // Stone Age materials
        let flint = Material::new("flint".to_string(), "Flint".to_string())
            .with_description("Sharp stone for making basic tools".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(4.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::None)
            .with_harvest_time(80)
            .with_drop_quantity(1, 2)
            .with_weight(0.3); // Flint chips are light

        let native_copper = Material::new("native_copper".to_string(), "Native Copper".to_string())
            .with_description("Pure copper nuggets - can be hammered cold".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(3.5)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::None)
            .with_harvest_time(120)
            .with_drop_quantity(1, 1)
            .with_weight(8.9) // Pure copper is very dense
            .with_melting_point(1085.0) // Copper melts at 1085°C
            .with_cold_working(); // Can be hammered cold!

        let copper_ore = Material::new("copper_ore".to_string(), "Copper Ore".to_string())
            .with_description("Green malachite ore containing copper".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(4.5)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::None)
            .with_harvest_time(150)
            .with_drop_quantity(1, 1)
            .with_weight(3.5) // Ore is lighter than pure metal
            .as_ore("copper_ingot".to_string(), 0.6); // 60% yield

        let tin_ore = Material::new("tin_ore".to_string(), "Tin Ore".to_string())
            .with_description("Cassiterite ore containing tin".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(4.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::None)
            .with_harvest_time(140)
            .with_drop_quantity(1, 1)
            .with_weight(6.0)
            .as_ore("tin_ingot".to_string(), 0.5); // 50% yield

        let lead_ore = Material::new("lead_ore".to_string(), "Lead Ore".to_string())
            .with_description("Galena ore containing lead".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(3.5)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::None)
            .with_harvest_time(130)
            .with_drop_quantity(1, 1)
            .with_weight(7.5)
            .with_melting_point(327.0) // Lead melts very easily!
            .as_ore("lead_ingot".to_string(), 0.7); // 70% yield

        // Processed metals
        let copper_ingot = Material::new("copper_ingot".to_string(), "Copper Ingot".to_string())
            .with_description("Smelted copper ingot".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(3.0)
            .with_stack_size(64)
            .with_weight(8.9)
            .with_melting_point(1085.0)
            .with_workable_temp(800.0); // Can be forged at red heat

        let tin_ingot = Material::new("tin_ingot".to_string(), "Tin Ingot".to_string())
            .with_description("Smelted tin ingot".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(2.0)
            .with_stack_size(64)
            .with_weight(7.3)
            .with_melting_point(232.0); // Tin melts easily

        let bronze_ingot = Material::new("bronze_ingot".to_string(), "Bronze Ingot".to_string())
            .with_description("Copper-tin alloy ingot".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(4.0)
            .with_stack_size(64)
            .with_weight(8.8)
            .with_melting_point(950.0) // Bronze melts lower than copper
            .with_workable_temp(750.0);

        let lead_ingot = Material::new("lead_ingot".to_string(), "Lead Ingot".to_string())
            .with_description("Smelted lead ingot - soft and heavy".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(1.5)
            .with_stack_size(64)
            .with_weight(11.3) // Lead is very dense
            .with_melting_point(327.0);

        // Stone Age tools
        let flint_knife = Material::new("flint_knife".to_string(), "Flint Knife".to_string())
            .with_description("Sharp flint blade".to_string())
            .with_category(MaterialCategory::Tool)
            .with_hardness(4.0)
            .with_tool_requirement(ToolType::Hand, ToolTier::None)
            .with_durability(30)
            .with_weight(0.2);

        let flint_axe = Material::new("flint_axe".to_string(), "Flint Axe".to_string())
            .with_description("Flint axe head on wooden handle".to_string())
            .with_category(MaterialCategory::Tool)
            .with_hardness(4.0)
            .with_tool_requirement(ToolType::Hand, ToolTier::None)
            .with_durability(40)
            .with_weight(1.0);

        let flint_spear = Material::new("flint_spear".to_string(), "Flint Spear".to_string())
            .with_description("Flint-tipped spear for hunting".to_string())
            .with_category(MaterialCategory::Tool)
            .with_hardness(3.5)
            .with_tool_requirement(ToolType::Hand, ToolTier::None)
            .with_durability(25)
            .with_weight(1.5);

        // Natural resources
        let wood = Material::new("wood".to_string(), "Wood".to_string())
            .with_description("Raw wood from trees".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(2.0)
            .with_tool_requirement(ToolType::Axe, ToolTier::None)
            .with_harvest_time(100)
            .with_drop_quantity(1, 3)
            .with_weight(0.5) // Wood logs are relatively light
            .as_fuel(300)
            .flammable();

        let stone = Material::new("stone".to_string(), "Stone".to_string())
            .with_description("Basic stone material".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(3.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Wooden)
            .with_harvest_time(150)
            .with_drop_quantity(1, 1)
            .with_weight(2.5); // Stone is heavy

        let iron_ore = Material::new("iron_ore".to_string(), "Iron Ore".to_string())
            .with_description("Raw iron ore that needs smelting".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(5.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Stone)
            .with_harvest_time(200)
            .with_drop_quantity(1, 1)
            .with_weight(7.0); // Iron ore is very heavy (metal ore)

        let coal = Material::new("coal".to_string(), "Coal".to_string())
            .with_description("Fuel source and crafting material".to_string())
            .with_category(MaterialCategory::Fuel)
            .with_hardness(3.0)
            .with_tool_requirement(ToolType::Pickaxe, ToolTier::Wooden)
            .with_harvest_time(100)
            .with_drop_quantity(1, 1)
            .with_weight(0.8) // Coal is lighter than stone
            .as_fuel(1600);

        // Processed materials
        let planks = Material::new("planks".to_string(), "Wooden Planks".to_string())
            .with_description("Processed wood for building".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(1.5)
            .with_weight(0.2) // Planks are lighter than logs
            .as_fuel(200)
            .flammable();

        let sticks = Material::new("sticks".to_string(), "Sticks".to_string())
            .with_description("Basic crafting component".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(0.5)
            .with_weight(0.05) // Sticks are very light
            .as_fuel(100);

        let iron_ingot = Material::new("iron_ingot".to_string(), "Iron Ingot".to_string())
            .with_description("Smelted iron for tools and equipment".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(4.0)
            .with_weight(5.0); // Dense metal ingot

        // Tools
        let wooden_pickaxe = Material::new("wooden_pickaxe".to_string(), "Wooden Pickaxe".to_string())
            .with_description("Basic mining tool".to_string())
            .with_category(MaterialCategory::Tool)
            .with_durability(60)
            .with_stack_size(1)
            .with_weight(1.5) // Tool weight includes handle and head
            .as_fuel(200);

        let stone_pickaxe = Material::new("stone_pickaxe".to_string(), "Stone Pickaxe".to_string())
            .with_description("Improved mining tool".to_string())
            .with_category(MaterialCategory::Tool)
            .with_durability(132)
            .with_stack_size(1)
            .with_weight(2.5); // Stone head is heavier

        let iron_pickaxe = Material::new("iron_pickaxe".to_string(), "Iron Pickaxe".to_string())
            .with_description("Advanced mining tool".to_string())
            .with_category(MaterialCategory::Tool)
            .with_durability(251)
            .with_stack_size(1)
            .with_weight(3.0); // Iron head is heaviest

        let wooden_axe = Material::new("wooden_axe".to_string(), "Wooden Axe".to_string())
            .with_description("Basic woodcutting tool".to_string())
            .with_category(MaterialCategory::Tool)
            .with_durability(60)
            .with_stack_size(1)
            .with_weight(1.2) // Axe is lighter than pickaxe
            .as_fuel(200);

        // Food
        let apple = Material::new("apple".to_string(), "Apple".to_string())
            .with_description("Restores hunger".to_string())
            .with_category(MaterialCategory::Food)
            .as_food(4.0)
            .with_stack_size(16)
            .with_weight(0.2); // Apples are light

        // Water (critical for life)
        let water = Material::new("water".to_string(), "Water".to_string())
            .with_description("Fresh water source - essential for survival".to_string())
            .with_category(MaterialCategory::Liquid)
            .with_hardness(0.0)
            .with_tool_requirement(ToolType::Hand, ToolTier::None)
            .with_harvest_time(20)
            .with_drop_quantity(1, 1)
            .with_stack_size(16)
            .with_weight(1.0); // 1kg per liter of water

        let dirt = Material::new("dirt".to_string(), "Dirt".to_string())
            .with_description("Basic soil material".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(0.5)
            .with_tool_requirement(ToolType::Shovel, ToolTier::None)
            .with_harvest_time(30)
            .with_drop_quantity(1, 1)
            .with_weight(1.2); // Soil is moderately heavy

        let grass = Material::new("grass".to_string(), "Grass".to_string())
            .with_description("Grass-covered dirt".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(0.6)
            .with_tool_requirement(ToolType::Shovel, ToolTier::None)
            .with_harvest_time(30)
            .with_drop_quantity(1, 1)
            .with_weight(1.2); // Same as dirt

        let sand = Material::new("sand".to_string(), "Sand".to_string())
            .with_description("Sandy material found near water".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(0.5)
            .with_tool_requirement(ToolType::Shovel, ToolTier::None)
            .with_harvest_time(25)
            .with_drop_quantity(1, 1)
            .with_weight(1.6); // Sand is denser than dirt

        // Water containers - for carrying water
        let leather = Material::new("leather".to_string(), "Leather".to_string())
            .with_description("Leather for crafting".to_string())
            .with_category(MaterialCategory::Processed)
            .with_hardness(1.0)
            .with_stack_size(16)
            .with_weight(0.3); // Leather is lightweight

        let mut waterskin = Material::new("waterskin".to_string(), "Leather Waterskin".to_string())
            .with_description("Basic water container - holds 8 units of water".to_string())
            .with_category(MaterialCategory::Container)
            .with_hardness(0.5)
            .with_stack_size(1)
            .with_weight(0.3); // Empty waterskin weight
        waterskin.properties.insert("capacity".to_string(), "8.0".to_string());

        let mut canteen = Material::new("canteen".to_string(), "Iron Canteen".to_string())
            .with_description("Improved water container - holds 16 units of water".to_string())
            .with_category(MaterialCategory::Container)
            .with_hardness(1.0)
            .with_stack_size(1)
            .with_weight(0.8); // Iron container is heavier
        canteen.properties.insert("capacity".to_string(), "16.0".to_string());

        let mut advanced_canteen = Material::new("advanced_canteen".to_string(), "Advanced Canteen".to_string())
            .with_description("High-capacity water container - holds 32 units of water".to_string())
            .with_category(MaterialCategory::Container)
            .with_hardness(1.5)
            .with_stack_size(1)
            .with_weight(1.2); // Larger iron container
        advanced_canteen.properties.insert("capacity".to_string(), "32.0".to_string());

        let bucket = Material::new("bucket".to_string(), "Iron Bucket".to_string())
            .with_description("Bucket for carrying water".to_string())
            .with_category(MaterialCategory::Tool)
            .with_hardness(1.0)
            .with_stack_size(16)
            .with_weight(1.0); // Iron bucket

        let clay = Material::new("clay".to_string(), "Clay".to_string())
            .with_description("Clay for building and crafting".to_string())
            .with_category(MaterialCategory::Natural)
            .with_hardness(0.6)
            .with_tool_requirement(ToolType::Shovel, ToolTier::None)
            .with_harvest_time(40)
            .with_drop_quantity(1, 1)
            .with_weight(1.8); // Clay is dense and heavy

        // Register all materials
        for material in vec![
            // Stone Age materials
            flint, native_copper, copper_ore, tin_ore, lead_ore,
            copper_ingot, tin_ingot, bronze_ingot, lead_ingot,
            flint_knife, flint_axe, flint_spear,
            // Original materials
            wood, stone, iron_ore, coal, planks, sticks, iron_ingot,
            wooden_pickaxe, stone_pickaxe, iron_pickaxe, wooden_axe, apple,
            water, dirt, grass, sand, leather, waterskin, canteen, advanced_canteen,
            bucket, clay,
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

        // Drink water
        let drink = Action::new(
            "drink_water".to_string(),
            "Drink Water".to_string(),
            ActionType::Eat, // Reusing Eat action type for consumption
        )
        .with_description("Drink water to restore thirst".to_string())
        .with_effects(
            ActionEffects::none()
                .with_time_cost(5)
                .with_drive_effect(DriveType::Thirst, -0.6),
        );

        // Fill water container from source
        let fill_container = Action::new(
            "fill_container".to_string(),
            "Fill Water Container".to_string(),
            ActionType::Store,
        )
        .with_description("Fill water containers from a water source or structure".to_string())
        .with_effects(
            ActionEffects::none()
                .with_energy_cost(2.0)
                .with_time_cost(30),
        );

        // Build structure
        let build_structure = Action::new(
            "build_structure".to_string(),
            "Build Structure".to_string(),
            ActionType::Build,
        )
        .with_description("Construct a building or structure".to_string())
        .with_effects(
            ActionEffects::none()
                .with_energy_cost(15.0)
                .with_time_cost(200)
                .with_drive_effect(DriveType::Construction, -0.3)
                .with_experience("construction".to_string(), 50.0),
        );

        // Draw water from structure
        let draw_water = Action::new(
            "draw_water".to_string(),
            "Draw Water".to_string(),
            ActionType::Retrieve,
        )
        .with_description("Draw water from a well, cistern, or water tower".to_string())
        .with_effects(
            ActionEffects::none()
                .with_energy_cost(1.0)
                .with_time_cost(15),
        );

        // Upgrade structure
        let upgrade_structure = Action::new(
            "upgrade_structure".to_string(),
            "Upgrade Structure".to_string(),
            ActionType::Build,
        )
        .with_description("Upgrade a structure to the next level".to_string())
        .with_effects(
            ActionEffects::none()
                .with_energy_cost(20.0)
                .with_time_cost(300)
                .with_drive_effect(DriveType::Construction, -0.4)
                .with_experience("construction".to_string(), 75.0),
        );

        // Register all actions
        for action in vec![chop_tree, mine_stone, craft, eat, drink, fill_container,
                           build_structure, draw_water, upgrade_structure] {
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

        // Water container recipes
        // Leather Waterskin
        let waterskin_recipe = CraftingTemplate::new(
            "waterskin".to_string(),
            "Leather Waterskin".to_string(),
        )
        .with_description("Craft a basic water container".to_string())
        .with_input(Ingredient::new("leather".to_string(), 2))
        .with_output(CraftingOutput::new("waterskin".to_string(), 1))
        .at_station(CraftingStation::Workbench)
        .with_craft_time(15)
        .with_energy_cost(3.0)
        .with_experience(5.0);

        // Iron Canteen
        let canteen_recipe = CraftingTemplate::new(
            "canteen".to_string(),
            "Iron Canteen".to_string(),
        )
        .with_description("Craft an improved water container".to_string())
        .with_input(Ingredient::new("iron_ingot".to_string(), 2))
        .with_output(CraftingOutput::new("canteen".to_string(), 1))
        .at_station(CraftingStation::Workbench)
        .with_craft_time(25)
        .with_energy_cost(5.0)
        .with_experience(15.0);

        // Advanced Canteen
        let advanced_canteen_recipe = CraftingTemplate::new(
            "advanced_canteen".to_string(),
            "Advanced Canteen".to_string(),
        )
        .with_description("Craft a high-capacity water container".to_string())
        .with_input(Ingredient::new("iron_ingot".to_string(), 3))
        .with_input(Ingredient::new("leather".to_string(), 1))
        .with_output(CraftingOutput::new("advanced_canteen".to_string(), 1))
        .at_station(CraftingStation::Workbench)
        .with_craft_time(35)
        .with_energy_cost(8.0)
        .with_experience(25.0);

        // Iron Bucket
        let bucket_recipe = CraftingTemplate::new(
            "bucket".to_string(),
            "Iron Bucket".to_string(),
        )
        .with_description("Craft a bucket for carrying water".to_string())
        .with_input(Ingredient::new("iron_ingot".to_string(), 3))
        .with_output(CraftingOutput::new("bucket".to_string(), 1))
        .at_station(CraftingStation::Workbench)
        .with_craft_time(20)
        .with_energy_cost(4.0)
        .with_experience(10.0);

        // Register all recipes
        for recipe in vec![
            planks_recipe,
            sticks_recipe,
            wooden_pickaxe_recipe,
            stone_pickaxe_recipe,
            iron_pickaxe_recipe,
            wooden_axe_recipe,
            iron_ingot_recipe,
            waterskin_recipe,
            canteen_recipe,
            advanced_canteen_recipe,
            bucket_recipe,
        ] {
            self.recipe_book.add_recipe(recipe);
        }
    }

    fn generate_world(&mut self) {
        let config = self.config.as_ref().unwrap();
        let (width, depth, max_height) = config.world_size;
        let mut rng = rand::thread_rng();

        // Initialize Perlin noise generators with different seeds for varied terrain
        let terrain_noise = Perlin::new(config.seed as u32);
        let moisture_noise = Perlin::new((config.seed + 1000) as u32);
        let cave_noise = Perlin::new((config.seed + 2000) as u32);

        // Terrain generation constants
        const SEA_LEVEL: i32 = 64;
        const BEACH_LEVEL: i32 = 66;
        const TERRAIN_SCALE: f64 = 0.02; // Smoother terrain
        const MOISTURE_SCALE: f64 = 0.03;
        const CAVE_SCALE: f64 = 0.1;
        const CAVE_THRESHOLD: f64 = 0.6;

        // Generate heightmap and terrain
        for x in -width/2..width/2 {
            for z in -depth/2..depth/2 {
                // Generate height using Perlin noise (0.0 to 1.0)
                let height_noise = terrain_noise.get([
                    x as f64 * TERRAIN_SCALE,
                    z as f64 * TERRAIN_SCALE,
                ]);

                // Convert to actual height (20 to 90)
                let base_height = ((height_noise + 1.0) / 2.0 * 70.0 + 20.0) as i32;

                // Generate moisture for biome determination
                let moisture = moisture_noise.get([
                    x as f64 * MOISTURE_SCALE,
                    z as f64 * MOISTURE_SCALE,
                ]);

                let is_wet = moisture > 0.0;
                let is_beach = base_height >= SEA_LEVEL && base_height <= BEACH_LEVEL;

                // Fill terrain from bedrock to surface
                for y in 0..=base_height {
                    // Check for caves using 3D noise
                    let cave_value = cave_noise.get([
                        x as f64 * CAVE_SCALE,
                        y as f64 * CAVE_SCALE,
                        z as f64 * CAVE_SCALE,
                    ]);

                    // Skip this block if it's a cave (but not near surface or below sea level)
                    if y > 10 && y < base_height - 3 && cave_value.abs() > CAVE_THRESHOLD {
                        continue;
                    }

                    let material = if y == 0 {
                        // Bedrock layer
                        "stone"
                    } else if y < base_height - 4 {
                        // Deep underground - stone with occasional ores
                        if y < 50 && rng.gen_bool(0.02) {
                            "coal"
                        } else if y < 40 && rng.gen_bool(0.008) {
                            "iron_ore"
                        } else {
                            "stone"
                        }
                    } else if y < base_height - 1 {
                        // Subsurface layer
                        if is_beach {
                            "sand"
                        } else {
                            "dirt"
                        }
                    } else if y == base_height {
                        // Surface layer
                        if is_beach {
                            "sand"
                        } else {
                            "grass"
                        }
                    } else {
                        continue;
                    };

                    self.world_map.insert(
                        Position::new(x, y, z),
                        material.to_string(),
                    );
                }

                // Add water to fill areas below sea level
                if base_height < SEA_LEVEL {
                    for y in (base_height + 1)..=SEA_LEVEL {
                        self.world_map.insert(
                            Position::new(x, y, z),
                            "water".to_string(),
                        );
                    }
                }

                // Add surface features
                if base_height >= SEA_LEVEL {
                    // Trees on grass (not on beaches)
                    if !is_beach && is_wet && rng.gen_bool(0.08) {
                        // Tree trunk (3-5 blocks tall)
                        let tree_height = rng.gen_range(3..=5);
                        for dy in 1..=tree_height {
                            self.world_map.insert(
                                Position::new(x, base_height + dy, z),
                                "wood".to_string(),
                            );
                        }
                    } else if !is_beach && !is_wet && rng.gen_bool(0.03) {
                        // Scattered trees in dry areas
                        self.world_map.insert(
                            Position::new(x, base_height + 1, z),
                            "wood".to_string(),
                        );
                    }
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
                        // Handle food
                        if material.is_edible {
                            result = result
                                .with_drive_change(DriveType::Hunger, -material.food_value * 0.1)
                                .with_item_consumed(ItemStack::new(material_id.clone(), 1));
                        }
                        // Handle water (drinkable liquid)
                        if material_id == "water" && material.category == MaterialCategory::Liquid {
                            result = result
                                .with_drive_change(DriveType::Thirst, -0.6)
                                .with_item_consumed(ItemStack::new(material_id, 1));
                        }
                    }
                }
            }
            ActionType::Store => {
                // Fill water containers from source or structure
                if action.id == "fill_container" {
                    result = result.with_message("Water containers filled".to_string());
                    // Note: Actual container filling would be handled by the agent's inventory system
                }
            }
            ActionType::Build => {
                // Build or upgrade structures
                if action.id == "build_structure" {
                    result = result.with_message("Structure construction in progress".to_string());
                    // Note: Structure construction would create a Structure instance
                } else if action.id == "upgrade_structure" {
                    result = result.with_message("Structure upgrade in progress".to_string());
                }
            }
            ActionType::Retrieve => {
                // Draw water from structures
                if action.id == "draw_water" {
                    result = result.with_message("Water drawn from structure".to_string());
                    // Note: Would check for nearby water storage structures
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
