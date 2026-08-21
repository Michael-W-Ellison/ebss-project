// src/world/nutrition.rs
//! Nutrition system for food, preparation states, and spoilage.
//!
//! Implements a realistic nutrition model with:
//! - Three nutrient types: Energy (carbs/fats), Protein, Micronutrients
//! - Food preparation states affecting utilization and spoilage
//! - Time-based food spoilage with preservation methods

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::inventory::ItemType;

/// Types of nutrients that agents need
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NutrientType {
    /// Carbohydrates/Fats - powers movement and metabolism
    Energy,
    /// Repairs tissues, enzyme function
    Protein,
    /// Vitamins/Minerals pooled together
    Micronutrients,
}

/// Nutritional content of a food item
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct NutritionalContent {
    /// Energy value (carbs/fats) - satisfies hunger, powers movement
    pub energy: f32,
    /// Protein content - repairs tissues
    pub protein: f32,
    /// Micronutrient content (vitamins/minerals)
    pub micronutrients: f32,
    /// Water content (0.0-1.0) - contributes to thirst satisfaction
    pub water_content: f32,
}

impl NutritionalContent {
    pub fn new(energy: f32, protein: f32, micronutrients: f32, water: f32) -> Self {
        Self {
            energy,
            protein,
            micronutrients,
            water_content: water,
        }
    }

    /// Total nutritional value (for hunger satisfaction calculations)
    pub fn total(&self) -> f32 {
        self.energy + self.protein + self.micronutrients
    }

    /// Scale all values by a factor
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            energy: self.energy * factor,
            protein: self.protein * factor,
            micronutrients: self.micronutrients * factor,
            water_content: self.water_content * factor,
        }
    }

    /// Check if this provides meaningful nutrition
    pub fn is_nutritious(&self) -> bool {
        self.total() > 5.0
    }
}

/// Preparation state of food affecting utilization and spoilage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PreparationState {
    /// Unprocessed food - low utilization, normal spoilage
    #[default]
    Raw,
    /// Heat-processed - high utilization, slightly slower spoilage
    Cooked,
    /// Dehydrated - good retention, very slow spoilage
    Dried,
    /// Smoke-preserved - good retention, very slow spoilage
    Smoked,
    /// Salt-preserved - moderate retention, slow spoilage
    Salted,
    /// Acid/brine preserved - moderate retention, very slow spoilage
    Pickled,
    /// Mechanically processed (flour, etc.) - high utilization, faster spoilage
    Ground,
    /// Fermented (cheese, ale) - enhanced nutrients, slow spoilage
    Fermented,
}

impl PreparationState {
    /// Get utilization multiplier (how much nutrition is absorbed)
    /// Raw foods are hard to digest; cooking/processing improves absorption
    pub fn utilization_multiplier(&self) -> f32 {
        match self {
            Self::Raw => 0.35,       // 30-40% utilization
            Self::Cooked => 0.95,    // 90-100% - cooking unlocks nutrients
            Self::Dried => 0.85,     // Good retention, some loss
            Self::Smoked => 0.80,    // Slight nutrient loss from heat
            Self::Salted => 0.75,    // Some nutrient loss
            Self::Pickled => 0.70,   // Fermentation changes profile
            Self::Ground => 0.90,    // Improved digestibility
            Self::Fermented => 0.85, // Enhanced some nutrients, lost others
        }
    }

    /// Get spoilage rate multiplier (lower = longer lasting)
    pub fn spoilage_multiplier(&self) -> f32 {
        match self {
            Self::Raw => 1.0,        // Baseline
            Self::Cooked => 0.8,     // Slightly slower than raw
            Self::Dried => 0.05,     // 20x longer - very slow
            Self::Smoked => 0.1,     // 10x longer - very slow
            Self::Salted => 0.15,    // ~7x longer
            Self::Pickled => 0.1,    // 10x longer
            Self::Ground => 1.2,     // Faster (more surface area)
            Self::Fermented => 0.2,  // 5x longer
        }
    }

    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Raw => "Raw",
            Self::Cooked => "Cooked",
            Self::Dried => "Dried",
            Self::Smoked => "Smoked",
            Self::Salted => "Salted",
            Self::Pickled => "Pickled",
            Self::Ground => "Ground",
            Self::Fermented => "Fermented",
        }
    }
}

/// Food-specific data attached to inventory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodData {
    /// Base nutritional values (before preparation adjustments)
    pub base_nutrition: NutritionalContent,
    /// Current preparation state
    pub preparation: PreparationState,
    /// Freshness (1.0 = fresh, 0.0 = spoiled)
    pub freshness: f32,
    /// Tick when item was created/harvested
    pub created_tick: u32,
    /// Base spoilage rate (ticks to reach 0 freshness from 1.0 at Raw state)
    pub base_spoilage_ticks: u32,
}

impl FoodData {
    pub fn new(
        base_nutrition: NutritionalContent,
        preparation: PreparationState,
        base_spoilage_ticks: u32,
        created_tick: u32,
    ) -> Self {
        Self {
            base_nutrition,
            preparation,
            freshness: 1.0,
            created_tick,
            base_spoilage_ticks,
        }
    }

    /// Get effective nutrition after preparation and freshness factors
    pub fn effective_nutrition(&self) -> NutritionalContent {
        let utilization = self.preparation.utilization_multiplier();
        let freshness_factor = self.freshness.max(0.0);

        NutritionalContent {
            energy: self.base_nutrition.energy * utilization * freshness_factor,
            protein: self.base_nutrition.protein * utilization * freshness_factor,
            micronutrients: self.base_nutrition.micronutrients * utilization * freshness_factor,
            water_content: self.base_nutrition.water_content * freshness_factor,
        }
    }

    /// Check if food is spoiled (inedible without consequences)
    pub fn is_spoiled(&self) -> bool {
        self.freshness <= 0.1
    }

    /// Whether this food has turned far enough to smell of it
    pub fn is_rotting(&self) -> bool {
        self.freshness < 0.4
    }

    /// How strongly this food gives itself away by smell, as a fraction of an
    /// agent's full smelling range.
    ///
    /// Cooking is the loudest thing a nose ever meets, which is why a camp can
    /// be smelled long before it is seen. Rot is the next loudest and carries
    /// most of the way. Anything raw and whole is close to silent.
    pub fn scent_strength(&self) -> f32 {
        if self.is_rotting() {
            // The further gone it is, the further it carries
            let rottenness = (0.4 - self.freshness.max(0.0)) / 0.4;
            return (0.35 + rottenness * 0.45).clamp(0.0, 0.8);
        }

        match self.preparation {
            // Food over a fire is unmistakable
            PreparationState::Cooked | PreparationState::Smoked => 1.0,
            // Prepared, but not hot
            PreparationState::Dried
            | PreparationState::Fermented
            | PreparationState::Ground
            | PreparationState::Salted
            | PreparationState::Pickled => 0.3,
            // Whole and raw: barely there
            PreparationState::Raw => 0.1,
        }
    }

    /// Check if food is harmful (causes sickness if eaten)
    pub fn is_harmful(&self) -> bool {
        self.freshness <= 0.0 ||
        (self.freshness < 0.3 && self.preparation == PreparationState::Raw)
    }

    /// Get freshness description
    pub fn freshness_description(&self) -> &'static str {
        match self.freshness {
            f if f > 0.8 => "Fresh",
            f if f > 0.5 => "Good",
            f if f > 0.3 => "Stale",
            f if f > 0.1 => "Spoiling",
            _ => "Spoiled",
        }
    }

    /// Update freshness based on current tick
    pub fn update_freshness(&mut self, current_tick: u32) {
        let elapsed = current_tick.saturating_sub(self.created_tick);
        let spoilage_rate = self.preparation.spoilage_multiplier();
        let effective_spoilage_ticks = (self.base_spoilage_ticks as f32 / spoilage_rate) as u32;

        if effective_spoilage_ticks > 0 {
            self.freshness = (1.0 - (elapsed as f32 / effective_spoilage_ticks as f32)).max(0.0);
        }
    }

    /// Change preparation state (e.g., cooking raw meat)
    /// Resets created_tick to current tick for spoilage calculations
    pub fn set_preparation(&mut self, new_state: PreparationState, current_tick: u32) {
        self.preparation = new_state;
        self.created_tick = current_tick;
        // Freshness resets when food is prepared
        self.freshness = 1.0;
    }
}

/// Template for creating food items - defines base nutritional values
#[derive(Debug, Clone)]
pub struct FoodTemplate {
    pub base_nutrition: NutritionalContent,
    /// Ticks to spoil at raw state
    pub base_spoilage_ticks: u32,
    /// Default preparation state when created
    pub default_preparation: PreparationState,
}

/// Registry of food nutritional data
pub struct FoodDatabase {
    entries: HashMap<ItemType, FoodTemplate>,
}

impl Default for FoodDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl FoodDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            entries: HashMap::new(),
        };
        db.register_all_foods();
        db
    }

    pub fn get(&self, item_type: &ItemType) -> Option<&FoodTemplate> {
        self.entries.get(item_type)
    }

    /// Create FoodData for an item type
    pub fn create_food_data(&self, item_type: &ItemType, current_tick: u32) -> Option<FoodData> {
        self.entries.get(item_type).map(|template| {
            FoodData::new(
                template.base_nutrition,
                template.default_preparation,
                template.base_spoilage_ticks,
                current_tick,
            )
        })
    }

    /// Check if an item type is food
    pub fn is_food(&self, item_type: &ItemType) -> bool {
        self.entries.contains_key(item_type)
    }

    fn register_all_foods(&mut self) {
        // === MEAT & FISH (High protein, moderate energy) ===

        // Meat - high protein, moderate energy, low micronutrients
        self.entries.insert(ItemType::Meat, FoodTemplate {
            base_nutrition: NutritionalContent::new(30.0, 50.0, 10.0, 0.6),
            base_spoilage_ticks: 1440, // 1 day raw
            default_preparation: PreparationState::Raw,
        });

        // Fish - high protein, moderate energy, good micronutrients (omega-3, etc.)
        self.entries.insert(ItemType::Fish, FoodTemplate {
            base_nutrition: NutritionalContent::new(25.0, 45.0, 20.0, 0.7),
            base_spoilage_ticks: 720, // 0.5 day - spoils very fast
            default_preparation: PreparationState::Raw,
        });

        // === GRAINS (High energy, low protein) ===

        // Grain - high energy, low protein, moderate micronutrients
        self.entries.insert(ItemType::Grain, FoodTemplate {
            base_nutrition: NutritionalContent::new(60.0, 15.0, 15.0, 0.1),
            base_spoilage_ticks: 14400, // 10 days - lasts long when dry
            default_preparation: PreparationState::Raw,
        });

        // Bread - processed grain, already cooked
        self.entries.insert(ItemType::Bread, FoodTemplate {
            base_nutrition: NutritionalContent::new(55.0, 12.0, 10.0, 0.3),
            base_spoilage_ticks: 2880, // 2 days
            default_preparation: PreparationState::Cooked,
        });

        // === DAIRY (Balanced nutrition) ===

        // Milk - balanced nutrition, high water
        self.entries.insert(ItemType::Milk, FoodTemplate {
            base_nutrition: NutritionalContent::new(25.0, 20.0, 25.0, 0.85),
            base_spoilage_ticks: 360, // 0.25 day - spoils very fast
            default_preparation: PreparationState::Raw,
        });

        // Cheese - preserved milk, concentrated nutrients
        self.entries.insert(ItemType::Cheese, FoodTemplate {
            base_nutrition: NutritionalContent::new(40.0, 35.0, 20.0, 0.35),
            base_spoilage_ticks: 10080, // 7 days
            default_preparation: PreparationState::Fermented,
        });

        // === SWEETS & BEVERAGES ===

        // Honey - pure energy, practically never spoils
        self.entries.insert(ItemType::Honey, FoodTemplate {
            base_nutrition: NutritionalContent::new(80.0, 0.0, 5.0, 0.2),
            base_spoilage_ticks: 100000, // Effectively never
            default_preparation: PreparationState::Raw, // Honey is special - raw but fully usable
        });

        // Ale - fermented grain beverage
        self.entries.insert(ItemType::Ale, FoodTemplate {
            base_nutrition: NutritionalContent::new(45.0, 5.0, 10.0, 0.9),
            base_spoilage_ticks: 20160, // 14 days
            default_preparation: PreparationState::Fermented,
        });

        // === GENERIC FOOD (Berries, foraged items) ===

        // Generic "Food" - represents berries, foraged items
        // High in micronutrients (vitamins from fruits/vegetables)
        self.entries.insert(ItemType::Food, FoodTemplate {
            base_nutrition: NutritionalContent::new(20.0, 5.0, 35.0, 0.8),
            base_spoilage_ticks: 2160, // 1.5 days
            default_preparation: PreparationState::Raw,
        });
    }
}

/// Agent's nutritional state - tracks nutrient reserves and deficiencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutritionalState {
    /// Energy reserves (carbs/fats) - 0-100
    /// Depletes with activity, restored by eating energy-rich foods
    pub energy_reserves: f32,

    /// Protein stores - 0-100
    /// Depletes slowly, needed for healing and maintenance
    pub protein_stores: f32,

    /// Micronutrient levels - 0-100
    /// Depletes very slowly, causes deficiency diseases when low
    pub micronutrient_level: f32,

    /// Ticks spent with protein below critical threshold
    pub ticks_protein_deficit: u32,

    /// Ticks spent with micronutrients below critical threshold
    pub ticks_micronutrient_deficit: u32,
}

impl Default for NutritionalState {
    fn default() -> Self {
        Self::new()
    }
}

impl NutritionalState {
    /// Critical threshold below which deficiency effects begin
    pub const DEFICIT_THRESHOLD: f32 = 20.0;
    /// Ticks of protein deficit before wasting begins (1 day)
    pub const PROTEIN_DEFICIT_ONSET: u32 = 1440;
    /// Ticks of micronutrient deficit before scurvy-like symptoms (3 days)
    pub const MICRONUTRIENT_DEFICIT_ONSET: u32 = 4320;

    pub fn new() -> Self {
        Self {
            energy_reserves: 80.0,
            protein_stores: 80.0,
            micronutrient_level: 80.0,
            ticks_protein_deficit: 0,
            ticks_micronutrient_deficit: 0,
        }
    }

    /// Create with full nutrition (for well-fed agents)
    pub fn full() -> Self {
        Self {
            energy_reserves: 100.0,
            protein_stores: 100.0,
            micronutrient_level: 100.0,
            ticks_protein_deficit: 0,
            ticks_micronutrient_deficit: 0,
        }
    }

    /// Consume nutrition from food
    pub fn consume(&mut self, nutrition: &NutritionalContent) {
        self.energy_reserves = (self.energy_reserves + nutrition.energy).min(100.0);
        self.protein_stores = (self.protein_stores + nutrition.protein).min(100.0);
        self.micronutrient_level = (self.micronutrient_level + nutrition.micronutrients).min(100.0);
    }

    /// Tick metabolism - depletes nutrients over time
    /// activity_level: 0.0 (resting) to 1.0 (intense activity)
    pub fn tick_metabolism(&mut self, activity_level: f32) {
        // Energy depletes faster with activity
        // Base: 0.02/tick at rest, up to 0.05/tick at full activity
        let energy_drain = 0.02 + (activity_level * 0.03);
        self.energy_reserves = (self.energy_reserves - energy_drain).max(0.0);

        // Protein depletes slowly (tissue maintenance)
        // ~100 ticks to deplete 0.5 points
        self.protein_stores = (self.protein_stores - 0.005).max(0.0);

        // Micronutrients deplete very slowly
        // ~100 ticks to deplete 0.2 points
        self.micronutrient_level = (self.micronutrient_level - 0.002).max(0.0);

        // Track deficiency duration
        if self.protein_stores < Self::DEFICIT_THRESHOLD {
            self.ticks_protein_deficit += 1;
        } else {
            self.ticks_protein_deficit = 0;
        }

        if self.micronutrient_level < Self::DEFICIT_THRESHOLD {
            self.ticks_micronutrient_deficit += 1;
        } else {
            self.ticks_micronutrient_deficit = 0;
        }
    }

    /// Check if experiencing protein deficiency (wasting)
    pub fn has_protein_deficiency(&self) -> bool {
        self.protein_stores < Self::DEFICIT_THRESHOLD &&
        self.ticks_protein_deficit > Self::PROTEIN_DEFICIT_ONSET
    }

    /// Check if experiencing micronutrient deficiency (scurvy-like)
    pub fn has_micronutrient_deficiency(&self) -> bool {
        self.micronutrient_level < Self::DEFICIT_THRESHOLD &&
        self.ticks_micronutrient_deficit > Self::MICRONUTRIENT_DEFICIT_ONSET
    }

    /// Get health penalty per tick from deficiencies
    pub fn deficiency_health_penalty(&self) -> f32 {
        let mut penalty = 0.0;

        // Protein deficiency causes wasting (progressive health loss)
        if self.has_protein_deficiency() {
            let days_in_deficit = (self.ticks_protein_deficit - Self::PROTEIN_DEFICIT_ONSET) as f32
                / 1440.0;
            // Scales from 0.05 to 0.15 health/tick over 3 days
            penalty += 0.05 * (1.0 + days_in_deficit.min(2.0));
        }

        // Micronutrient deficiency causes disease symptoms
        if self.has_micronutrient_deficiency() {
            let days_in_deficit = (self.ticks_micronutrient_deficit - Self::MICRONUTRIENT_DEFICIT_ONSET) as f32
                / 1440.0;
            // Scales from 0.02 to 0.06 health/tick over 3 days
            penalty += 0.02 * (1.0 + days_in_deficit.min(2.0));
        }

        penalty
    }

    /// Get overall nutritional status (0.0 = critical, 1.0 = excellent)
    pub fn overall_status(&self) -> f32 {
        let energy_factor = self.energy_reserves / 100.0;
        let protein_factor = self.protein_stores / 100.0;
        let micro_factor = self.micronutrient_level / 100.0;

        // Weighted average - energy is most important short-term
        energy_factor * 0.5 + protein_factor * 0.3 + micro_factor * 0.2
    }

    /// Check if agent is starving (critical energy)
    pub fn is_starving(&self) -> bool {
        self.energy_reserves < 10.0
    }

    /// Get the most deficient nutrient type
    pub fn most_needed_nutrient(&self) -> NutrientType {
        if self.energy_reserves <= self.protein_stores &&
           self.energy_reserves <= self.micronutrient_level {
            NutrientType::Energy
        } else if self.protein_stores <= self.micronutrient_level {
            NutrientType::Protein
        } else {
            NutrientType::Micronutrients
        }
    }

    /// Get status string for debugging/display
    pub fn status_string(&self) -> String {
        let mut status = format!(
            "E:{:.0} P:{:.0} V:{:.0}",
            self.energy_reserves,
            self.protein_stores,
            self.micronutrient_level
        );

        if self.has_protein_deficiency() {
            status.push_str(" [WASTING]");
        }
        if self.has_micronutrient_deficiency() {
            status.push_str(" [SCURVY]");
        }

        status
    }
}

/// Result of eating food
#[derive(Debug, Clone)]
pub enum EatResult {
    /// Successfully consumed food with given nutrition
    Success(NutritionalContent),
    /// Food was too spoiled to eat
    Spoiled,
    /// Ate bad food and got sick
    MadeSick(f32), // Health damage taken
    /// No food available
    NoFood,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preparation_utilization() {
        assert!(PreparationState::Cooked.utilization_multiplier() >
                PreparationState::Raw.utilization_multiplier());
        assert_eq!(PreparationState::Raw.utilization_multiplier(), 0.35);
        assert_eq!(PreparationState::Cooked.utilization_multiplier(), 0.95);
    }

    #[test]
    fn test_preparation_spoilage() {
        // Dried food should spoil much slower
        assert!(PreparationState::Dried.spoilage_multiplier() <
                PreparationState::Raw.spoilage_multiplier());
        // Ground food spoils faster
        assert!(PreparationState::Ground.spoilage_multiplier() >
                PreparationState::Raw.spoilage_multiplier());
    }

    #[test]
    fn test_food_data_effective_nutrition() {
        let food = FoodData::new(
            NutritionalContent::new(100.0, 50.0, 25.0, 0.5),
            PreparationState::Raw,
            1000,
            0,
        );

        let effective = food.effective_nutrition();
        // Raw utilization is 35%
        assert!((effective.energy - 35.0).abs() < 0.01);
        assert!((effective.protein - 17.5).abs() < 0.01);
    }

    #[test]
    fn test_food_data_cooked_nutrition() {
        let food = FoodData::new(
            NutritionalContent::new(100.0, 50.0, 25.0, 0.5),
            PreparationState::Cooked,
            1000,
            0,
        );

        let effective = food.effective_nutrition();
        // Cooked utilization is 95%
        assert!((effective.energy - 95.0).abs() < 0.01);
    }

    #[test]
    fn test_food_freshness() {
        let mut food = FoodData::new(
            NutritionalContent::new(100.0, 50.0, 25.0, 0.5),
            PreparationState::Raw,
            1000, // Spoils in 1000 ticks
            0,
        );

        assert_eq!(food.freshness_description(), "Fresh");
        assert!(!food.is_spoiled());

        // Simulate 500 ticks passing (50% fresh)
        food.update_freshness(500);
        assert!((food.freshness - 0.5).abs() < 0.01);
        // At exactly 0.5, it's not > 0.5, so it's "Stale"
        assert_eq!(food.freshness_description(), "Stale");

        // Simulate 800 ticks - should be spoiling (20% fresh)
        food.update_freshness(800);
        assert!(food.freshness < 0.25);
        assert!(food.freshness > 0.1);
        assert_eq!(food.freshness_description(), "Spoiling");

        // Simulate 1000+ ticks - should be spoiled
        food.update_freshness(1100);
        assert!(food.is_spoiled());
    }

    #[test]
    fn test_dried_food_lasts_longer() {
        let mut raw_food = FoodData::new(
            NutritionalContent::new(50.0, 50.0, 10.0, 0.6),
            PreparationState::Raw,
            1000,
            0,
        );

        let mut dried_food = FoodData::new(
            NutritionalContent::new(50.0, 50.0, 10.0, 0.6),
            PreparationState::Dried,
            1000,
            0,
        );

        // After 1000 ticks, raw should be nearly spoiled
        raw_food.update_freshness(1000);
        dried_food.update_freshness(1000);

        // Dried food should still be mostly fresh (20x slower spoilage)
        assert!(dried_food.freshness > 0.9);
        assert!(raw_food.freshness < 0.1);
    }

    #[test]
    fn test_nutritional_state_metabolism() {
        let mut state = NutritionalState::full();

        // Tick 100 times at moderate activity
        for _ in 0..100 {
            state.tick_metabolism(0.5);
        }

        // Energy should have depleted most
        assert!(state.energy_reserves < 100.0);
        assert!(state.energy_reserves < state.protein_stores);
        assert!(state.protein_stores < 100.0);
    }

    #[test]
    fn test_protein_deficiency() {
        let mut state = NutritionalState {
            energy_reserves: 50.0,
            protein_stores: 10.0, // Below threshold
            micronutrient_level: 50.0,
            ticks_protein_deficit: 0,
            ticks_micronutrient_deficit: 0,
        };

        // Not deficient yet - need time
        assert!(!state.has_protein_deficiency());

        // Simulate 1.5 days of deficit
        state.ticks_protein_deficit = 2000;
        assert!(state.has_protein_deficiency());
        assert!(state.deficiency_health_penalty() > 0.0);
    }

    #[test]
    fn test_food_database() {
        let db = FoodDatabase::new();

        assert!(db.is_food(&ItemType::Meat));
        assert!(db.is_food(&ItemType::Bread));
        assert!(!db.is_food(&ItemType::Wood));

        let meat = db.get(&ItemType::Meat).unwrap();
        assert!(meat.base_nutrition.protein > meat.base_nutrition.energy);

        let grain = db.get(&ItemType::Grain).unwrap();
        assert!(grain.base_nutrition.energy > grain.base_nutrition.protein);
    }

    #[test]
    fn test_consume_nutrition() {
        let mut state = NutritionalState::new();
        let initial_energy = state.energy_reserves;

        let nutrition = NutritionalContent::new(20.0, 10.0, 5.0, 0.5);
        state.consume(&nutrition);

        assert!((state.energy_reserves - (initial_energy + 20.0)).abs() < 0.01);
        assert!((state.protein_stores - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_most_needed_nutrient() {
        let state = NutritionalState {
            energy_reserves: 30.0,
            protein_stores: 50.0,
            micronutrient_level: 80.0,
            ticks_protein_deficit: 0,
            ticks_micronutrient_deficit: 0,
        };

        assert_eq!(state.most_needed_nutrient(), NutrientType::Energy);
    }

    #[test]
    fn test_honey_never_spoils() {
        let db = FoodDatabase::new();
        let honey = db.get(&ItemType::Honey).unwrap();

        // Honey has very long spoilage time
        assert!(honey.base_spoilage_ticks > 50000);

        let mut food = db.create_food_data(&ItemType::Honey, 0).unwrap();
        food.update_freshness(5000); // 5000 ticks later

        // Still fresh (at 5% degradation, should be 0.95)
        assert!(food.freshness > 0.9);
        assert_eq!(food.freshness_description(), "Fresh");
    }
}
