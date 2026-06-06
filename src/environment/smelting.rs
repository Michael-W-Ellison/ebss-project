// src/environment/smelting.rs
//! Smelting recipe system for ore-to-metal transformations
//!
//! Integrates with heat sources to transform raw ores into usable metals.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A smelting recipe defining transformation of raw material to refined product
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmeltingRecipe {
    pub id: String,
    pub name: String,
    pub description: String,

    /// Input material (ore, raw material, etc.)
    pub input_material: String,
    /// How much input is needed
    pub input_quantity: u32,

    /// Secondary input material (for alloys like bronze = copper + tin)
    pub secondary_input: Option<(String, u32)>,

    /// Output material (ingot, metal, etc.)
    pub output_material: String,
    /// How much output is produced
    pub output_quantity: u32,

    /// Minimum temperature required (°C)
    pub melting_point: f32,
    /// How long to smelt (ticks)
    pub smelting_time: u32,

    /// Fuel cost multiplier (harder materials need more fuel)
    pub fuel_cost_multiplier: f32,
}

impl SmeltingRecipe {
    pub fn new(
        id: String,
        name: String,
        input_material: String,
        output_material: String,
        melting_point: f32,
    ) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            input_material,
            input_quantity: 1,
            secondary_input: None,
            output_material,
            output_quantity: 1,
            melting_point,
            smelting_time: 100, // Default 100 ticks
            fuel_cost_multiplier: 1.0,
        }
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    pub fn with_quantities(mut self, input: u32, output: u32) -> Self {
        self.input_quantity = input;
        self.output_quantity = output;
        self
    }

    /// Add a secondary input material (for alloys)
    pub fn with_secondary_input(mut self, material: String, quantity: u32) -> Self {
        self.secondary_input = Some((material, quantity));
        self
    }

    pub fn with_time(mut self, ticks: u32) -> Self {
        self.smelting_time = ticks;
        self
    }

    pub fn with_fuel_cost(mut self, multiplier: f32) -> Self {
        self.fuel_cost_multiplier = multiplier;
        self
    }

    /// Check if this recipe requires a secondary input
    pub fn requires_secondary(&self) -> bool {
        self.secondary_input.is_some()
    }
}

/// Smelting recipe registry
#[derive(Debug, Clone)]
pub struct SmeltingRegistry {
    recipes: HashMap<String, SmeltingRecipe>,
    /// Index by input material for quick lookup
    by_input: HashMap<String, Vec<String>>, // material_id -> recipe_ids
}

impl SmeltingRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            recipes: HashMap::new(),
            by_input: HashMap::new(),
        };
        registry.register_all_recipes();
        registry
    }

    fn register(&mut self, recipe: SmeltingRecipe) {
        // Add to input index
        self.by_input
            .entry(recipe.input_material.clone())
            .or_default()
            .push(recipe.id.clone());

        // Add to recipes
        self.recipes.insert(recipe.id.clone(), recipe);
    }

    pub fn get(&self, id: &str) -> Option<&SmeltingRecipe> {
        self.recipes.get(id)
    }

    /// Get all recipes that use a specific input material
    pub fn get_by_input(&self, input_material: &str) -> Vec<&SmeltingRecipe> {
        if let Some(recipe_ids) = self.by_input.get(input_material) {
            recipe_ids
                .iter()
                .filter_map(|id| self.recipes.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if a material can be smelted
    pub fn can_smelt(&self, material_id: &str) -> bool {
        self.by_input.contains_key(material_id)
    }

    pub fn all_recipes(&self) -> Vec<&SmeltingRecipe> {
        self.recipes.values().collect()
    }

    fn register_all_recipes(&mut self) {
        // ===== Copper Smelting =====
        // Copper melts at 1085°C, achievable with smelting fire (1000-1200°C)

        self.register(
            SmeltingRecipe::new(
                "smelt_copper".to_string(),
                "Smelt Copper".to_string(),
                "copper_ore".to_string(),
                "copper_ingot".to_string(),
                1085.0,
            )
            .with_description("Smelt copper ore into copper ingots".to_string())
            .with_quantities(2, 1) // 2 ore -> 1 ingot (50% yield)
            .with_time(150)
            .with_fuel_cost(1.2),
        );

        // ===== Tin Smelting =====
        // Tin melts at 232°C, very easy to smelt even in campfires

        self.register(
            SmeltingRecipe::new(
                "smelt_tin".to_string(),
                "Smelt Tin".to_string(),
                "tin_ore".to_string(),
                "tin_ingot".to_string(),
                232.0,
            )
            .with_description("Smelt tin ore into tin ingots".to_string())
            .with_quantities(2, 1)
            .with_time(80)
            .with_fuel_cost(0.5),
        );

        // ===== Bronze Creation =====
        // Bronze is an alloy (not technically smelting, but similar process)
        // Requires copper + tin at copper's melting point

        // Bronze alloy: 9 copper + 1 tin = 10 bronze
        // Historical bronze was typically 88% copper, 12% tin
        self.register(
            SmeltingRecipe::new(
                "make_bronze".to_string(),
                "Make Bronze".to_string(),
                "copper_ingot".to_string(),
                "bronze_ingot".to_string(),
                1085.0,
            )
            .with_description("Alloy copper with tin to create bronze".to_string())
            .with_quantities(9, 10) // 9 copper + 1 tin -> 10 bronze
            .with_secondary_input("tin_ingot".to_string(), 1) // Requires tin
            .with_time(120)
            .with_fuel_cost(1.0),
        );

        // ===== Iron Smelting =====
        // Iron melting point is 1538°C, but bloomery process works at 1200-1400°C
        // Produces iron bloom (spongy iron) not molten iron

        self.register(
            SmeltingRecipe::new(
                "smelt_iron_bloom".to_string(),
                "Smelt Iron (Bloomery)".to_string(),
                "iron_ore".to_string(),
                "iron_bloom".to_string(),
                1200.0,
            )
            .with_description("Reduce iron ore to iron bloom in a bloomery".to_string())
            .with_quantities(3, 1) // 3 ore -> 1 bloom (33% yield, historical accuracy)
            .with_time(300) // Iron smelting takes much longer
            .with_fuel_cost(2.0),
        );

        // Refining bloom to wrought iron (requires hammering but we'll abstract it)
        self.register(
            SmeltingRecipe::new(
                "refine_iron_bloom".to_string(),
                "Refine Iron Bloom".to_string(),
                "iron_bloom".to_string(),
                "iron_ingot".to_string(),
                800.0, // Reheating for working
            )
            .with_description("Refine iron bloom into wrought iron ingot".to_string())
            .with_quantities(2, 1) // 2 bloom -> 1 ingot (compacting and removing slag)
            .with_time(200)
            .with_fuel_cost(1.5),
        );

        // Direct iron smelting (requires advanced furnace at 1500°C+)
        self.register(
            SmeltingRecipe::new(
                "smelt_iron_direct".to_string(),
                "Smelt Iron (Direct)".to_string(),
                "iron_ore".to_string(),
                "iron_ingot".to_string(),
                1538.0,
            )
            .with_description("Directly smelt iron ore to liquid iron".to_string())
            .with_quantities(2, 1) // Better yield with advanced tech
            .with_time(250)
            .with_fuel_cost(2.5),
        );

        // ===== Steel Creation =====
        // Steel requires adding carbon to iron at high temperature
        // Simplified: iron + charcoal -> steel

        self.register(
            SmeltingRecipe::new(
                "make_steel".to_string(),
                "Make Steel".to_string(),
                "iron_ingot".to_string(),
                "steel_ingot".to_string(),
                1400.0,
            )
            .with_description("Carburize iron to create steel".to_string())
            .with_quantities(2, 1) // 2 iron + carbon -> 1 steel
            .with_time(400) // Steel making is slow
            .with_fuel_cost(3.0),
        );

        // ===== Gold Smelting =====
        // Gold melts at 1064°C, similar to copper

        self.register(
            SmeltingRecipe::new(
                "smelt_gold".to_string(),
                "Smelt Gold".to_string(),
                "gold_ore".to_string(),
                "gold_ingot".to_string(),
                1064.0,
            )
            .with_description("Smelt gold ore into pure gold ingots".to_string())
            .with_quantities(1, 1) // Gold has good yield
            .with_time(120)
            .with_fuel_cost(1.0),
        );

        // ===== Silver Smelting =====
        // Silver melts at 962°C

        self.register(
            SmeltingRecipe::new(
                "smelt_silver".to_string(),
                "Smelt Silver".to_string(),
                "silver_ore".to_string(),
                "silver_ingot".to_string(),
                962.0,
            )
            .with_description("Smelt silver ore into silver ingots".to_string())
            .with_quantities(2, 1)
            .with_time(140)
            .with_fuel_cost(1.1),
        );

        // ===== Lead Smelting =====
        // Lead melts at 327°C, very easy (can accidentally melt in campfires!)

        self.register(
            SmeltingRecipe::new(
                "smelt_lead".to_string(),
                "Smelt Lead".to_string(),
                "lead_ore".to_string(),
                "lead_ingot".to_string(),
                327.0,
            )
            .with_description("Smelt lead ore (caution: toxic fumes!)".to_string())
            .with_quantities(1, 1)
            .with_time(60)
            .with_fuel_cost(0.4),
        );

        // ===== Glass Making =====
        // Glass from sand requires 1400-1600°C

        self.register(
            SmeltingRecipe::new(
                "make_glass".to_string(),
                "Make Glass".to_string(),
                "sand".to_string(),
                "glass".to_string(),
                1400.0,
            )
            .with_description("Melt sand into glass".to_string())
            .with_quantities(3, 1)
            .with_time(180)
            .with_fuel_cost(1.8),
        );

        // ===== Brick Making =====
        // Firing clay into bricks at 900-1000°C

        self.register(
            SmeltingRecipe::new(
                "fire_brick".to_string(),
                "Fire Brick".to_string(),
                "clay".to_string(),
                "brick".to_string(),
                900.0,
            )
            .with_description("Fire clay to create bricks".to_string())
            .with_quantities(1, 1)
            .with_time(100)
            .with_fuel_cost(0.8),
        );

        // ===== Charcoal Production =====
        // Charcoal from wood (pyrolysis, low oxygen)

        self.register(
            SmeltingRecipe::new(
                "make_charcoal".to_string(),
                "Make Charcoal".to_string(),
                "wood".to_string(),
                "charcoal".to_string(),
                400.0, // Low temp, slow burn
            )
            .with_description("Convert wood to charcoal through pyrolysis".to_string())
            .with_quantities(4, 1) // 4 wood -> 1 charcoal
            .with_time(200)
            .with_fuel_cost(0.3), // Uses less fuel as it's self-sustaining
        );
    }
}

impl Default for SmeltingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of checking if smelting is possible
#[derive(Debug, Clone)]
pub enum SmeltingCheck {
    CanSmelt {
        recipe_id: String,
        output_material: String,
        output_quantity: u32,
    },
    NoRecipe,
    TemperatureTooLow {
        required: f32,
        current: f32,
    },
    InsufficientTime {
        required: u32,
        current: u32,
    },
}

/// Select the best recipe from a list based on current conditions
///
/// Prioritizes:
/// 1. Recipes achievable at current temperature
/// 2. Higher output quantity (better yield)
/// 3. Lower time requirement (efficiency)
fn select_best_recipe<'a>(recipes: &[&'a SmeltingRecipe], current_temp: f32) -> &'a SmeltingRecipe {
    // Filter to recipes that can be done at current temperature
    let achievable: Vec<_> = recipes.iter()
        .filter(|r| current_temp >= r.melting_point)
        .collect();

    // If some are achievable, pick the best one
    if !achievable.is_empty() {
        // Score by: output_quantity / input_quantity / time
        // Higher is better
        achievable.into_iter()
            .max_by(|a, b| {
                let score_a = (a.output_quantity as f32 / a.input_quantity as f32) / a.smelting_time as f32;
                let score_b = (b.output_quantity as f32 / b.input_quantity as f32) / b.smelting_time as f32;
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap()
    } else {
        // No achievable recipes - return the one with lowest temp requirement
        recipes.iter()
            .min_by(|a, b| a.melting_point.partial_cmp(&b.melting_point).unwrap_or(std::cmp::Ordering::Equal))
            .copied()
            .unwrap()
    }
}

/// Check if material is ready to smelt using the best available recipe
pub fn check_smelting(
    registry: &SmeltingRegistry,
    material_id: &str,
    heating_time: u32,
    current_temp: f32,
) -> SmeltingCheck {
    check_smelting_with_recipe(registry, material_id, None, heating_time, current_temp)
}

/// Check if material is ready to smelt with optional recipe selection
///
/// If recipe_id is None, selects the best recipe based on:
/// 1. Temperature requirements (recipes achievable at current temp)
/// 2. Output efficiency (higher output quantity preferred)
/// 3. Time efficiency (faster recipes preferred when temps are similar)
pub fn check_smelting_with_recipe(
    registry: &SmeltingRegistry,
    material_id: &str,
    recipe_id: Option<&str>,
    heating_time: u32,
    current_temp: f32,
) -> SmeltingCheck {
    // Find recipes for this material
    let recipes = registry.get_by_input(material_id);

    if recipes.is_empty() {
        return SmeltingCheck::NoRecipe;
    }

    // Select recipe: either by ID or choose the best one
    let recipe = if let Some(id) = recipe_id {
        match recipes.iter().find(|r| r.id == id) {
            Some(r) => r,
            None => return SmeltingCheck::NoRecipe,
        }
    } else {
        // Select best recipe based on current conditions
        select_best_recipe(&recipes, current_temp)
    };

    // Check temperature
    if current_temp < recipe.melting_point {
        return SmeltingCheck::TemperatureTooLow {
            required: recipe.melting_point,
            current: current_temp,
        };
    }

    // Check time
    if heating_time < recipe.smelting_time {
        return SmeltingCheck::InsufficientTime {
            required: recipe.smelting_time,
            current: heating_time,
        };
    }

    // Ready to smelt!
    SmeltingCheck::CanSmelt {
        recipe_id: recipe.id.clone(),
        output_material: recipe.output_material.clone(),
        output_quantity: recipe.output_quantity,
    }
}
