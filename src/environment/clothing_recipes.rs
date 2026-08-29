// src/environment/clothing_recipes.rs
//! Crafting recipes for clothing and armor.

use super::crafting::{CraftingTemplate, Ingredient, CraftingOutput, CraftingStation};
use super::MaterialCategory;
use std::collections::BTreeMap;

/// Get all clothing crafting recipes
pub fn clothing_recipes() -> Vec<CraftingTemplate> {
    vec![
        leather_tunic_recipe(),
        leather_pants_recipe(),
        leather_gloves_recipe(),
        fur_coat_recipe(),
        fur_hat_recipe(),
        wool_cloak_recipe(),
        linen_shirt_recipe(),
        hide_armor_recipe(),
        bark_boots_recipe(),
    ]
}

/// Leather tunic - basic torso protection
fn leather_tunic_recipe() -> CraftingTemplate {
    CraftingTemplate {
        id: "leather_tunic".to_string(),
        name: "Leather Tunic".to_string(),
        description: "Basic leather chest protection with moderate insulation".to_string(),
        category: MaterialCategory::Clothing,
        inputs: vec![
            Ingredient::new("leather".to_string(), 8),
            Ingredient::new("thread".to_string(), 4),
        ],
        outputs: vec![
            CraftingOutput::new("leather_tunic".to_string(), 1),
        ],
        required_station: CraftingStation::Workbench,
        required_tool: None,
        required_tier: None,
        craft_time: 120,
        energy_cost: 10.0,
        experience_gain: 5.0,
        required_skill: None,
        discoverable: true,
        unlocked: true,
        properties: BTreeMap::new(),
    }
}

/// Leather pants - leg protection
fn leather_pants_recipe() -> CraftingTemplate {
    CraftingTemplate {
        id: "leather_pants".to_string(),
        name: "Leather Pants".to_string(),
        description: "Durable leather leg protection".to_string(),
        category: MaterialCategory::Clothing,
        inputs: vec![
            Ingredient::new("leather".to_string(), 6),
            Ingredient::new("thread".to_string(), 3),
        ],
        outputs: vec![
            CraftingOutput::new("leather_pants".to_string(), 1),
        ],
        required_station: CraftingStation::Workbench,
        required_tool: None,
        required_tier: None,
        craft_time: 90,
        energy_cost: 8.0,
        experience_gain: 4.0,
        required_skill: None,
        discoverable: true,
        unlocked: true,
        properties: BTreeMap::new(),
    }
}

/// Leather gloves - arm protection
fn leather_gloves_recipe() -> CraftingTemplate {
    CraftingTemplate {
        id: "leather_gloves".to_string(),
        name: "Leather Gloves".to_string(),
        description: "Flexible leather hand protection".to_string(),
        category: MaterialCategory::Clothing,
        inputs: vec![
            Ingredient::new("leather".to_string(), 3),
            Ingredient::new("thread".to_string(), 2),
        ],
        outputs: vec![
            CraftingOutput::new("leather_gloves".to_string(), 1),
        ],
        required_station: CraftingStation::Workbench,
        required_tool: None,
        required_tier: None,
        craft_time: 60,
        energy_cost: 5.0,
        experience_gain: 3.0,
        required_skill: None,
        discoverable: true,
        unlocked: true,
        properties: BTreeMap::new(),
    }
}

/// Fur coat - excellent cold weather protection
fn fur_coat_recipe() -> CraftingTemplate {
    CraftingTemplate {
        id: "fur_coat".to_string(),
        name: "Fur Coat".to_string(),
        description: "Thick fur coat for extreme cold weather".to_string(),
        category: MaterialCategory::Clothing,
        inputs: vec![
            Ingredient::new("fur".to_string(), 12),
            Ingredient::new("leather".to_string(), 4),
            Ingredient::new("thread".to_string(), 6),
        ],
        outputs: vec![
            CraftingOutput::new("fur_coat".to_string(), 1),
        ],
        required_station: CraftingStation::Workbench,
        required_tool: None,
        required_tier: None,
        craft_time: 180,
        energy_cost: 15.0,
        experience_gain: 8.0,
        required_skill: None,
        discoverable: true,
        unlocked: true,
        properties: BTreeMap::new(),
    }
}

/// Fur hat - head protection from cold
fn fur_hat_recipe() -> CraftingTemplate {
    CraftingTemplate {
        id: "fur_hat".to_string(),
        name: "Fur Hat".to_string(),
        description: "Warm fur hat for cold climates".to_string(),
        category: MaterialCategory::Clothing,
        inputs: vec![
            Ingredient::new("fur".to_string(), 4),
            Ingredient::new("thread".to_string(), 2),
        ],
        outputs: vec![
            CraftingOutput::new("fur_hat".to_string(), 1),
        ],
        required_station: CraftingStation::Workbench,
        required_tool: None,
        required_tier: None,
        craft_time: 45,
        energy_cost: 4.0,
        experience_gain: 2.0,
        required_skill: None,
        discoverable: true,
        unlocked: true,
        properties: BTreeMap::new(),
    }
}

/// Wool cloak - good all-around protection
fn wool_cloak_recipe() -> CraftingTemplate {
    CraftingTemplate {
        id: "wool_cloak".to_string(),
        name: "Wool Cloak".to_string(),
        description: "Comfortable wool cloak for moderate climates".to_string(),
        category: MaterialCategory::Clothing,
        inputs: vec![
            Ingredient::new("wool".to_string(), 10),
            Ingredient::new("thread".to_string(), 4),
        ],
        outputs: vec![
            CraftingOutput::new("wool_cloak".to_string(), 1),
        ],
        required_station: CraftingStation::Loom,
        required_tool: None,
        required_tier: None,
        craft_time: 150,
        energy_cost: 12.0,
        experience_gain: 6.0,
        required_skill: None,
        discoverable: true,
        unlocked: true,
        properties: BTreeMap::new(),
    }
}

/// Linen shirt - hot weather clothing
fn linen_shirt_recipe() -> CraftingTemplate {
    CraftingTemplate {
        id: "linen_shirt".to_string(),
        name: "Linen Shirt".to_string(),
        description: "Light, breathable shirt for hot weather".to_string(),
        category: MaterialCategory::Clothing,
        inputs: vec![
            Ingredient::new("linen".to_string(), 5),
            Ingredient::new("thread".to_string(), 2),
        ],
        outputs: vec![
            CraftingOutput::new("linen_shirt".to_string(), 1),
        ],
        required_station: CraftingStation::Loom,
        required_tool: None,
        required_tier: None,
        craft_time: 75,
        energy_cost: 6.0,
        experience_gain: 3.0,
        required_skill: None,
        discoverable: true,
        unlocked: true,
        properties: BTreeMap::new(),
    }
}

/// Hide armor - heavy protection
fn hide_armor_recipe() -> CraftingTemplate {
    CraftingTemplate {
        id: "hide_armor".to_string(),
        name: "Hide Armor".to_string(),
        description: "Thick hide armor providing excellent protection".to_string(),
        category: MaterialCategory::Clothing,
        inputs: vec![
            Ingredient::new("thick_hide".to_string(), 15),
            Ingredient::new("leather".to_string(), 6),
            Ingredient::new("thread".to_string(), 8),
        ],
        outputs: vec![
            CraftingOutput::new("hide_armor".to_string(), 1),
        ],
        required_station: CraftingStation::Workbench,
        required_tool: None,
        required_tier: None,
        craft_time: 240,
        energy_cost: 20.0,
        experience_gain: 10.0,
        required_skill: None,
        discoverable: true,
        unlocked: true,
        properties: BTreeMap::new(),
    }
}

/// Bark boots - primitive footwear
fn bark_boots_recipe() -> CraftingTemplate {
    CraftingTemplate {
        id: "bark_boots".to_string(),
        name: "Bark Boots".to_string(),
        description: "Simple boots made from bark and plant fibers".to_string(),
        category: MaterialCategory::Clothing,
        inputs: vec![
            Ingredient::new("bark".to_string(), 8),
            Ingredient::new("plant_fiber".to_string(), 6),
        ],
        outputs: vec![
            CraftingOutput::new("bark_boots".to_string(), 1),
        ],
        required_station: CraftingStation::None,
        required_tool: None,
        required_tier: None,
        craft_time: 30,
        energy_cost: 3.0,
        experience_gain: 1.0,
        required_skill: None,
        discoverable: true,
        unlocked: true,
        properties: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_clothing_recipes() {
        let recipes = clothing_recipes();
        assert_eq!(recipes.len(), 9);

        // Verify each recipe has required fields
        for recipe in recipes {
            assert!(!recipe.id.is_empty());
            assert!(!recipe.name.is_empty());
            assert!(!recipe.inputs.is_empty());
            assert!(!recipe.outputs.is_empty());
            assert!(recipe.craft_time > 0);
        }
    }

    #[test]
    fn test_leather_tunic_recipe() {
        let recipe = leather_tunic_recipe();
        assert_eq!(recipe.id, "leather_tunic");
        assert_eq!(recipe.inputs.len(), 2);
        assert_eq!(recipe.outputs.len(), 1);
        assert_eq!(recipe.required_station, CraftingStation::Workbench);
    }

    #[test]
    fn test_fur_coat_requires_multiple_materials() {
        let recipe = fur_coat_recipe();
        assert!(recipe.inputs.len() >= 3);

        // Should require fur, leather, and thread
        let material_ids: Vec<String> = recipe.inputs.iter().map(|i| i.material_id.clone()).collect();
        assert!(material_ids.contains(&"fur".to_string()));
        assert!(material_ids.contains(&"leather".to_string()));
        assert!(material_ids.contains(&"thread".to_string()));
    }

    #[test]
    fn test_bark_boots_no_station_required() {
        let recipe = bark_boots_recipe();
        assert_eq!(recipe.required_station, CraftingStation::None);
        assert!(recipe.craft_time < 60); // Quick to craft
    }

    #[test]
    fn test_wool_cloak_requires_loom() {
        let recipe = wool_cloak_recipe();
        assert_eq!(recipe.required_station, CraftingStation::Loom);
    }
}
