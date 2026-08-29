// src/environment/crafting.rs
//! Crafting system for creating items from materials.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use super::{ToolType, ToolTier, MaterialCategory};
use crate::agents::{Quality, skills::RecycledMaterial};

/// Represents a crafting station or workspace
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CraftingStation {
    /// No station required (hand crafting)
    None,
    /// Crafting table/workbench
    Workbench,
    /// Furnace for smelting
    Furnace,
    /// Anvil for metalworking
    Anvil,
    /// Loom for textiles
    Loom,
    /// Alchemy station
    AlchemyTable,
    /// Custom station (plugin-specific)
    Custom(String),
}

/// A crafting ingredient
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ingredient {
    /// Material ID
    pub material_id: String,
    /// Quantity required
    pub quantity: u32,
    /// Whether this ingredient is consumed in crafting
    pub consumed: bool,
}

impl Ingredient {
    pub fn new(material_id: String, quantity: u32) -> Self {
        Self {
            material_id,
            quantity,
            consumed: true,
        }
    }

    pub fn tool(material_id: String) -> Self {
        Self {
            material_id,
            quantity: 1,
            consumed: false,
        }
    }
}

/// Output of a crafting recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftingOutput {
    /// Material ID of the output
    pub material_id: String,
    /// Quantity produced
    pub quantity: u32,
    /// Chance of success (0.0 to 1.0, 1.0 = always succeeds)
    pub success_chance: f32,
}

impl CraftingOutput {
    pub fn new(material_id: String, quantity: u32) -> Self {
        Self {
            material_id,
            quantity,
            success_chance: 1.0,
        }
    }

    pub fn with_chance(mut self, chance: f32) -> Self {
        self.success_chance = chance.clamp(0.0, 1.0);
        self
    }
}

/// A crafting recipe template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftingTemplate {
    /// Unique identifier for this recipe
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Category (for UI organization)
    pub category: MaterialCategory,

    /// Input ingredients
    pub inputs: Vec<Ingredient>,
    /// Output items
    pub outputs: Vec<CraftingOutput>,

    /// Required crafting station
    pub required_station: CraftingStation,
    /// Required tool (if any)
    pub required_tool: Option<ToolType>,
    /// Required tool tier (if any)
    pub required_tier: Option<ToolTier>,

    /// Crafting time in ticks
    pub craft_time: u32,
    /// Energy cost to craft
    pub energy_cost: f32,
    /// Experience gained from crafting
    pub experience_gain: f32,
    /// Skill required to craft (skill_name, min_level)
    pub required_skill: Option<(String, f32)>,

    /// Whether this recipe is discoverable through experimentation
    pub discoverable: bool,
    /// Whether the player has unlocked this recipe
    pub unlocked: bool,

    /// Custom properties
    pub properties: BTreeMap<String, String>,
}

impl CraftingTemplate {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            category: MaterialCategory::Processed,
            inputs: Vec::new(),
            outputs: Vec::new(),
            required_station: CraftingStation::None,
            required_tool: None,
            required_tier: None,
            craft_time: 20,
            energy_cost: 5.0,
            experience_gain: 10.0,
            required_skill: None,
            discoverable: false,
            unlocked: true,
            properties: BTreeMap::new(),
        }
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    pub fn with_category(mut self, category: MaterialCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_input(mut self, ingredient: Ingredient) -> Self {
        self.inputs.push(ingredient);
        self
    }

    pub fn with_output(mut self, output: CraftingOutput) -> Self {
        self.outputs.push(output);
        self
    }

    pub fn at_station(mut self, station: CraftingStation) -> Self {
        self.required_station = station;
        self
    }

    pub fn with_tool(mut self, tool: ToolType, tier: ToolTier) -> Self {
        self.required_tool = Some(tool);
        self.required_tier = Some(tier);
        self
    }

    pub fn with_craft_time(mut self, time: u32) -> Self {
        self.craft_time = time;
        self
    }

    pub fn with_energy_cost(mut self, cost: f32) -> Self {
        self.energy_cost = cost;
        self
    }

    pub fn with_experience(mut self, exp: f32) -> Self {
        self.experience_gain = exp;
        self
    }


    pub fn discoverable(mut self) -> Self {
        self.discoverable = true;
        self.unlocked = false;
        self
    }

    pub fn with_property(mut self, key: String, value: String) -> Self {
        self.properties.insert(key, value);
        self
    }

    /// Check if agent has the required materials
    pub fn has_materials(&self, inventory: &BTreeMap<String, u32>) -> bool {
        self.inputs.iter().all(|ingredient| {
            inventory
                .get(&ingredient.material_id)
                .map(|&qty| qty >= ingredient.quantity)
                .unwrap_or(false)
        })
    }


}

/// Recipe book for managing discovered recipes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeBook {
    /// All recipes in the game
    recipes: BTreeMap<String, CraftingTemplate>,
    /// Recipes discovered by agent
    discovered: BTreeMap<String, bool>,
}

impl RecipeBook {
    pub fn new() -> Self {
        Self {
            recipes: BTreeMap::new(),
            discovered: BTreeMap::new(),
        }
    }

    pub fn add_recipe(&mut self, recipe: CraftingTemplate) {
        let id = recipe.id.clone();
        let auto_unlock = recipe.unlocked;
        self.recipes.insert(id.clone(), recipe);
        if auto_unlock {
            self.discovered.insert(id, true);
        }
    }

    pub fn discover_recipe(&mut self, recipe_id: &str) {
        self.discovered.insert(recipe_id.to_string(), true);
    }

    pub fn is_discovered(&self, recipe_id: &str) -> bool {
        self.discovered.get(recipe_id).copied().unwrap_or(false)
    }

    pub fn get_recipe(&self, recipe_id: &str) -> Option<&CraftingTemplate> {
        self.recipes.get(recipe_id)
    }

    pub fn available_recipes(&self) -> Vec<&CraftingTemplate> {
        self.recipes
            .iter()
            .filter(|(id, _)| self.is_discovered(id))
            .map(|(_, recipe)| recipe)
            .collect()
    }

    pub fn craftable_recipes(&self, inventory: &BTreeMap<String, u32>) -> Vec<&CraftingTemplate> {
        self.available_recipes()
            .into_iter()
            .filter(|recipe| recipe.has_materials(inventory))
            .collect()
    }
}

impl Default for RecipeBook {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate repair cost for an item
/// Formula: (material_cost - durability_percentage) * 0.2
///
/// # Arguments
/// * `recipe` - The crafting recipe for the item
/// * `current_durability_pct` - Current durability as percentage (0.0 to 1.0)
///
/// # Returns
/// Vector of ingredients needed for repair
pub fn calculate_repair_cost(recipe: &CraftingTemplate, current_durability_pct: f32) -> Vec<Ingredient> {
    let durability_pct = current_durability_pct.clamp(0.0, 1.0);

    recipe.inputs.iter()
        .filter(|ing| ing.consumed) // Only consumed ingredients
        .map(|ing| {
            // (quantity * (1.0 - durability_pct)) * 0.2
            let loss_amount = (ing.quantity as f32) * (1.0 - durability_pct);
            let repair_cost = (loss_amount * 0.2).ceil() as u32; // Round up

            Ingredient {
                material_id: ing.material_id.clone(),
                quantity: repair_cost.max(1), // Minimum 1 if any repair needed
                consumed: true,
            }
        })
        .filter(|ing| ing.quantity > 0)
        .collect()
}

/// Calculate materials returned from recycling an item
///
/// Durability tiers:
/// - 0% (broken): 20% materials, -2 quality levels
/// - 1-49%: 50% materials, -1 quality level
/// - 50-100%: 75% materials, same quality
///
/// # Arguments
/// * `recipe` - The crafting recipe for the item
/// * `current_durability_pct` - Current durability as percentage (0.0 to 1.0)
/// * `item_quality` - Quality of the item being recycled
///
/// # Returns
/// Vector of recycled materials with adjusted quantities and qualities
pub fn calculate_recycle_returns(
    recipe: &CraftingTemplate,
    current_durability_pct: f32,
    item_quality: Quality,
) -> Vec<RecycledMaterial> {
    let durability_pct = current_durability_pct.clamp(0.0, 1.0);

    // Determine return percentage and quality downgrade
    let (return_pct, quality_downgrade) = if durability_pct == 0.0 {
        (0.2, 2) // 20%, -2 quality
    } else if durability_pct < 0.5 {
        (0.5, 1) // 50%, -1 quality
    } else {
        (0.75, 0) // 75%, same quality
    };

    let recycled_quality = item_quality.downgrade(quality_downgrade);

    recipe.inputs.iter()
        .filter(|ing| ing.consumed) // Only consumed ingredients
        .map(|ing| {
            let returned_quantity = ((ing.quantity as f32) * return_pct).floor() as u32;

            RecycledMaterial {
                material_id: ing.material_id.clone(),
                quantity: returned_quantity,
                quality: recycled_quality,
            }
        })
        .filter(|mat| mat.quantity > 0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingredient_creation() {
        let ingredient = Ingredient::new("wood".to_string(), 4);
        assert_eq!(ingredient.material_id, "wood");
        assert_eq!(ingredient.quantity, 4);
        assert!(ingredient.consumed);

        let tool = Ingredient::tool("axe".to_string());
        assert!(!tool.consumed);
    }

    #[test]
    fn test_crafting_template() {
        let recipe = CraftingTemplate::new("wooden_planks".to_string(), "Wooden Planks".to_string())
            .with_input(Ingredient::new("wood".to_string(), 1))
            .with_output(CraftingOutput::new("planks".to_string(), 4))
            .with_craft_time(10)
            .at_station(CraftingStation::Workbench);

        assert_eq!(recipe.inputs.len(), 1);
        assert_eq!(recipe.outputs.len(), 1);
        assert_eq!(recipe.craft_time, 10);
    }

    #[test]
    fn test_has_materials() {
        let recipe = CraftingTemplate::new("test".to_string(), "Test".to_string())
            .with_input(Ingredient::new("wood".to_string(), 4))
            .with_input(Ingredient::new("stone".to_string(), 2));

        let mut inventory = BTreeMap::new();
        inventory.insert("wood".to_string(), 5);
        inventory.insert("stone".to_string(), 2);

        assert!(recipe.has_materials(&inventory));

        inventory.insert("wood".to_string(), 3);
        assert!(!recipe.has_materials(&inventory));
    }

    #[test]
    fn test_recipe_book() {
        let mut book = RecipeBook::new();

        let recipe = CraftingTemplate::new("planks".to_string(), "Planks".to_string())
            .with_input(Ingredient::new("wood".to_string(), 1))
            .with_output(CraftingOutput::new("planks".to_string(), 4));

        book.add_recipe(recipe);
        book.discover_recipe("planks");

        assert!(book.is_discovered("planks"));
        assert_eq!(book.available_recipes().len(), 1);
    }

    #[test]
    fn test_craftable_recipes() {
        let mut book = RecipeBook::new();

        let recipe1 = CraftingTemplate::new("planks".to_string(), "Planks".to_string())
            .with_input(Ingredient::new("wood".to_string(), 1))
            .with_output(CraftingOutput::new("planks".to_string(), 4));

        let recipe2 = CraftingTemplate::new("sticks".to_string(), "Sticks".to_string())
            .with_input(Ingredient::new("planks".to_string(), 2))
            .with_output(CraftingOutput::new("sticks".to_string(), 4));

        book.add_recipe(recipe1);
        book.add_recipe(recipe2);
        book.discover_recipe("planks");
        book.discover_recipe("sticks");

        let mut inventory = BTreeMap::new();
        inventory.insert("wood".to_string(), 5);

        let craftable = book.craftable_recipes(&inventory);
        assert_eq!(craftable.len(), 1); // Only planks is craftable
        assert_eq!(craftable[0].id, "planks");
    }

    #[test]
    fn test_repair_cost_calculation() {
        // Create iron axe recipe: 100 wood + 100 iron
        let recipe = CraftingTemplate::new("iron_axe".to_string(), "Iron Axe".to_string())
            .with_input(Ingredient::new("wood".to_string(), 100))
            .with_input(Ingredient::new("iron".to_string(), 100));

        // At 80% durability
        let repair_cost = calculate_repair_cost(&recipe, 0.8);

        // (100 - 80) * 0.2 = 4 for each material
        assert_eq!(repair_cost.len(), 2);
        assert_eq!(repair_cost[0].quantity, 4);
        assert_eq!(repair_cost[1].quantity, 4);
    }

    #[test]
    fn test_repair_cost_at_50_percent() {
        let recipe = CraftingTemplate::new("tool".to_string(), "Tool".to_string())
            .with_input(Ingredient::new("iron".to_string(), 50));

        let repair_cost = calculate_repair_cost(&recipe, 0.5);

        // (50 - 25) * 0.2 = 5
        assert_eq!(repair_cost[0].quantity, 5);
    }

    #[test]
    fn test_repair_cost_at_low_durability() {
        let recipe = CraftingTemplate::new("tool".to_string(), "Tool".to_string())
            .with_input(Ingredient::new("iron".to_string(), 100));

        let repair_cost = calculate_repair_cost(&recipe, 0.1);

        // (100 - 10) * 0.2 = 18
        assert_eq!(repair_cost[0].quantity, 18);
    }

    #[test]
    fn test_recycle_broken_item() {
        // Broken item (0% durability): 20% return, -2 quality
        let recipe = CraftingTemplate::new("tool".to_string(), "Tool".to_string())
            .with_input(Ingredient::new("iron".to_string(), 100))
            .with_input(Ingredient::new("wood".to_string(), 50));

        let recycled = calculate_recycle_returns(&recipe, 0.0, Quality::Advanced);

        // 20% of materials
        assert_eq!(recycled[0].quantity, 20); // 100 * 0.2
        assert_eq!(recycled[1].quantity, 10); // 50 * 0.2

        // Quality downgraded by 2 (Advanced=4 -> Basic=2)
        assert_eq!(recycled[0].quality, Quality::Basic);
        assert_eq!(recycled[1].quality, Quality::Basic);
    }

    #[test]
    fn test_recycle_damaged_item() {
        // Damaged item (30% durability): 50% return, -1 quality
        let recipe = CraftingTemplate::new("tool".to_string(), "Tool".to_string())
            .with_input(Ingredient::new("iron".to_string(), 100));

        let recycled = calculate_recycle_returns(&recipe, 0.3, Quality::Moderate);

        // 50% of materials
        assert_eq!(recycled[0].quantity, 50);

        // Quality downgraded by 1 (Moderate -> Basic)
        assert_eq!(recycled[0].quality, Quality::Basic);
    }

    #[test]
    fn test_recycle_good_condition_item() {
        // Good condition (70% durability): 75% return, same quality
        let recipe = CraftingTemplate::new("tool".to_string(), "Tool".to_string())
            .with_input(Ingredient::new("iron".to_string(), 100));

        let recycled = calculate_recycle_returns(&recipe, 0.7, Quality::Expert);

        // 75% of materials
        assert_eq!(recycled[0].quantity, 75);

        // Quality unchanged
        assert_eq!(recycled[0].quality, Quality::Expert);
    }

    #[test]
    fn test_recycle_filters_zero_quantity() {
        // Small recipe that would produce < 1 item on some returns
        let recipe = CraftingTemplate::new("small_tool".to_string(), "Small Tool".to_string())
            .with_input(Ingredient::new("iron".to_string(), 3));

        // Broken: 20% of 3 = 0.6, floors to 0, should be filtered out
        let recycled = calculate_recycle_returns(&recipe, 0.0, Quality::Basic);

        assert_eq!(recycled.len(), 0); // No materials returned
    }
}
