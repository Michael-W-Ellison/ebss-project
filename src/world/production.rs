// src/world/production.rs
//! Production and crafting system with skill-based recipes.

use serde::{Deserialize, Serialize};
use crate::world::{ResourceType, ItemType};

/// Quality tier of a crafted item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quality {
    Poor,       // 0-20 skill
    Common,     // 21-40 skill
    Good,       // 41-60 skill
    Excellent,  // 61-80 skill
    Masterwork, // 81-100 skill
}

impl Quality {
    /// Get quality from skill level
    pub fn from_skill(skill: u8) -> Self {
        match skill {
            0..=20 => Quality::Poor,
            21..=40 => Quality::Common,
            41..=60 => Quality::Good,
            61..=80 => Quality::Excellent,
            _ => Quality::Masterwork,
        }
    }

    /// Get output quantity multiplier
    pub fn output_multiplier(&self) -> f32 {
        match self {
            Quality::Poor => 0.8,
            Quality::Common => 1.0,
            Quality::Good => 1.2,
            Quality::Excellent => 1.4,
            Quality::Masterwork => 1.6,
        }
    }

    /// Get production time multiplier (lower is faster)
    pub fn time_multiplier(&self) -> f32 {
        match self {
            Quality::Poor => 1.2,       // 20% slower
            Quality::Common => 1.0,     // Normal speed
            Quality::Good => 0.85,      // 15% faster
            Quality::Excellent => 0.7,  // 30% faster
            Quality::Masterwork => 0.5, // 50% faster
        }
    }
}

/// A resource requirement for a recipe
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRequirement {
    pub resource_type: ResourceType,
    pub amount: u32,
}

impl ResourceRequirement {
    pub fn new(resource_type: ResourceType, amount: u32) -> Self {
        Self { resource_type, amount }
    }
}

/// A production output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductionOutput {
    pub item_type: ItemType,
    pub base_amount: u32,
}

impl ProductionOutput {
    pub fn new(item_type: ItemType, base_amount: u32) -> Self {
        Self { item_type, base_amount }
    }
}

/// A crafting recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub name: &'static str,
    pub inputs: Vec<ResourceRequirement>,
    pub outputs: Vec<ProductionOutput>,
    pub base_time: u32, // Base time in ticks
}

impl Recipe {
    /// Calculate actual output based on quality
    pub fn calculate_output(&self, quality: Quality) -> Vec<(ItemType, u32)> {
        self.outputs
            .iter()
            .map(|output| {
                let amount = (output.base_amount as f32 * quality.output_multiplier()).ceil() as u32;
                (output.item_type, amount.max(1))
            })
            .collect()
    }

    /// Calculate production time based on quality
    pub fn calculate_time(&self, quality: Quality) -> u32 {
        (self.base_time as f32 * quality.time_multiplier()).ceil() as u32
    }
}

// Note: Job-based recipe organization has been removed in favor of skill-based crafting.
// See src/world/crafting.rs for the new skill-based crafting system.
// This file is kept for legacy Recipe, Quality, and related types used by analytics.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_from_skill() {
        assert_eq!(Quality::from_skill(10), Quality::Poor);
        assert_eq!(Quality::from_skill(30), Quality::Common);
        assert_eq!(Quality::from_skill(50), Quality::Good);
        assert_eq!(Quality::from_skill(70), Quality::Excellent);
        assert_eq!(Quality::from_skill(90), Quality::Masterwork);
    }

    #[test]
    fn test_quality_multipliers() {
        let poor = Quality::Poor;
        assert_eq!(poor.output_multiplier(), 0.8);
        assert!(poor.time_multiplier() > 1.0);

        let master = Quality::Masterwork;
        assert_eq!(master.output_multiplier(), 1.6);
        assert!(master.time_multiplier() < 1.0);
    }

    #[test]
    fn test_recipe_calculation() {
        let recipe = Recipe {
            name: "Test Recipe",
            inputs: vec![ResourceRequirement::new(ResourceType::Flour, 1)],
            outputs: vec![ProductionOutput::new(ItemType::Bread, 2)],
            base_time: 100,
        };

        // Poor quality produces less
        let poor_output = recipe.calculate_output(Quality::Poor);
        assert_eq!(poor_output[0].1, 2); // 2 * 0.8 = 1.6 -> 2 (ceil)

        // Masterwork produces more
        let master_output = recipe.calculate_output(Quality::Masterwork);
        assert_eq!(master_output[0].1, 4); // 2 * 1.6 = 3.2 -> 4 (ceil)

        // Time calculation
        let poor_time = recipe.calculate_time(Quality::Poor);
        assert!(poor_time > recipe.base_time);

        let master_time = recipe.calculate_time(Quality::Masterwork);
        assert!(master_time < recipe.base_time);
    }
}
