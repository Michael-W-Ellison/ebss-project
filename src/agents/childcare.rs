// src/agents/childcare.rs
//! Childcare and nursing system for infant agents.

use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Duration of nursing period in ticks (infant stage)
pub const NURSING_DURATION: u32 = 500;

/// Maximum distance from caregiver before infant suffers
pub const MAX_CAREGIVER_DISTANCE: f32 = 10.0;

/// Health loss per tick when not nursed
pub const UNNURSED_HEALTH_LOSS: f32 = 0.5;

/// Energy restored per nursing tick
pub const NURSING_ENERGY_GAIN: f32 = 5.0;

/// Developmental nutrition tracking for early life stages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalNutrition {
    /// Nutrition quality during pregnancy (inherited from mother)
    pub prenatal_quality: f32,
    /// Nutrition quality during infant stage (0-500 ticks)
    pub infant_quality: f32,
    /// Nutrition quality during child stage (500-2500 ticks)
    pub child_quality: f32,
    /// Number of samples for infant stage
    infant_samples: u32,
    /// Number of samples for child stage
    child_samples: u32,
    /// Whether developmental stats have been finalized
    pub finalized: bool,
    /// Final stat modifiers (calculated when transitioning to adult)
    pub stat_modifiers: StatModifiers,
}

/// Permanent stat modifiers from developmental nutrition
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct StatModifiers {
    /// Modifier to max health (0.5 to 1.2)
    pub max_health: f32,
    /// Modifier to max energy (0.5 to 1.2)
    pub max_energy: f32,
    /// Modifier to fertility (0.3 to 1.1)
    pub fertility: f32,
    /// Modifier to learning rate (0.5 to 1.3)
    pub learning_rate: f32,
    /// Modifier to base strength (0.6 to 1.2)
    pub strength: f32,
}

impl Default for DevelopmentalNutrition {
    fn default() -> Self {
        Self {
            prenatal_quality: 1.0,
            infant_quality: 1.0,
            child_quality: 1.0,
            infant_samples: 0,
            child_samples: 0,
            finalized: false,
            stat_modifiers: StatModifiers::default(),
        }
    }
}

impl DevelopmentalNutrition {
    /// Create with inherited prenatal nutrition quality
    pub fn with_prenatal(prenatal_quality: f32) -> Self {
        Self {
            prenatal_quality: prenatal_quality.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Update infant nutrition quality
    /// Called each tick during infant stage
    pub fn update_infant_nutrition(&mut self, hunger_satisfaction: f32, was_nursed: bool) {
        if self.finalized {
            return;
        }

        let quality_this_tick = if was_nursed {
            // Nursing provides excellent nutrition
            0.9 + (hunger_satisfaction * 0.1)
        } else {
            // Not nursed - reduced nutrition even if fed
            hunger_satisfaction * 0.6
        };

        self.infant_samples += 1;
        let weight = 1.0 / self.infant_samples as f32;
        self.infant_quality = self.infant_quality * (1.0 - weight) + quality_this_tick * weight;
    }

    /// Update child nutrition quality
    /// Called each tick during child stage
    pub fn update_child_nutrition(&mut self, hunger_satisfaction: f32, health_percentage: f32) {
        if self.finalized {
            return;
        }

        let quality_this_tick = hunger_satisfaction * (health_percentage / 100.0);

        self.child_samples += 1;
        let weight = 1.0 / self.child_samples as f32;
        self.child_quality = self.child_quality * (1.0 - weight) + quality_this_tick * weight;
    }

    /// Finalize developmental stats when transitioning to adult
    /// This calculates permanent stat modifiers based on nutrition history
    /// Returns true if severe malnutrition caused infertility
    pub fn finalize(&mut self) -> bool {
        if self.finalized {
            return false;
        }

        // Weight different stages (prenatal and infant most important)
        let overall_quality =
            self.prenatal_quality * 0.35 +
            self.infant_quality * 0.40 +
            self.child_quality * 0.25;

        // Calculate modifiers based on overall nutrition quality
        // Poor nutrition (0.0-0.3): severe penalties
        // Average nutrition (0.3-0.7): mild effects
        // Good nutrition (0.7-1.0): bonuses
        self.stat_modifiers = StatModifiers {
            max_health: Self::calculate_modifier(overall_quality, 0.5, 1.2),
            max_energy: Self::calculate_modifier(overall_quality, 0.5, 1.2),
            fertility: Self::calculate_modifier(overall_quality, 0.3, 1.1),
            learning_rate: Self::calculate_modifier(overall_quality, 0.5, 1.3),
            strength: Self::calculate_modifier(overall_quality, 0.6, 1.2),
        };

        self.finalized = true;

        // Severe malnutrition (quality < 0.2) has a chance to cause permanent infertility
        // The worse the nutrition, the higher the chance (up to 30% at quality 0)
        if overall_quality < 0.2 {
            use rand::Rng;
            let mut rng = crate::core::dice::roll();
            let infertility_chance = (0.2 - overall_quality) * 1.5; // 0% at 0.2, 30% at 0
            if rng.gen_bool(infertility_chance as f64) {
                return true; // Agent becomes infertile
            }
        }

        false
    }

    /// Calculate a modifier value based on nutrition quality
    fn calculate_modifier(quality: f32, min_mod: f32, max_mod: f32) -> f32 {
        // Map quality (0.0-1.0) to modifier range (min_mod-max_mod)
        // Use a curve that penalizes poor nutrition more than it rewards good
        let curved_quality = if quality < 0.5 {
            // Steeper penalty curve for poor nutrition
            quality * quality * 2.0
        } else {
            // Gentler bonus curve for good nutrition
            0.5 + (quality - 0.5) * (quality - 0.5) * 2.0 + (quality - 0.5)
        };

        min_mod + curved_quality.clamp(0.0, 1.0) * (max_mod - min_mod)
    }

    /// Get a summary description of developmental nutrition
    pub fn summary(&self) -> &'static str {
        let avg = (self.prenatal_quality + self.infant_quality + self.child_quality) / 3.0;
        if avg >= 0.85 {
            "Excellent development"
        } else if avg >= 0.7 {
            "Good development"
        } else if avg >= 0.5 {
            "Average development"
        } else if avg >= 0.3 {
            "Poor development"
        } else {
            "Severely malnourished"
        }
    }
}

/// Nursing state for infants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NursingState {
    /// Primary caregiver (usually mother)
    pub primary_caregiver: Uuid,
    /// Secondary caregivers who can also nurse
    pub secondary_caregivers: Vec<Uuid>,
    /// Tick when nursing period ends
    pub nursing_end_tick: u32,
    /// Ticks since last nursed
    pub ticks_since_nursed: u32,
    /// Whether currently being nursed
    pub is_nursing: bool,
}

impl NursingState {
    /// Create new nursing state for a newborn
    pub fn new(birth_tick: u32, mother_id: Uuid) -> Self {
        Self {
            primary_caregiver: mother_id,
            secondary_caregivers: Vec::new(),
            nursing_end_tick: birth_tick + NURSING_DURATION,
            ticks_since_nursed: 0,
            is_nursing: false,
        }
    }

    /// Check if nursing period is still active
    pub fn needs_nursing(&self, current_tick: u32) -> bool {
        current_tick < self.nursing_end_tick
    }

    /// Add a secondary caregiver
    pub fn add_caregiver(&mut self, caregiver_id: Uuid) {
        if !self.secondary_caregivers.contains(&caregiver_id)
            && caregiver_id != self.primary_caregiver {
            self.secondary_caregivers.push(caregiver_id);
        }
    }

    /// Check if an agent is a valid caregiver
    pub fn is_caregiver(&self, agent_id: Uuid) -> bool {
        agent_id == self.primary_caregiver
            || self.secondary_caregivers.contains(&agent_id)
    }

    /// Record a nursing tick
    pub fn nurse(&mut self) {
        self.ticks_since_nursed = 0;
        self.is_nursing = true;
    }

    /// Record a tick without nursing
    pub fn tick_without_nursing(&mut self) {
        self.ticks_since_nursed += 1;
        self.is_nursing = false;
    }

    /// Check if infant is suffering from lack of nursing
    pub fn is_suffering(&self) -> bool {
        self.ticks_since_nursed > 10
    }

    /// Get health penalty for lack of nursing
    pub fn health_penalty(&self) -> f32 {
        if self.ticks_since_nursed <= 10 {
            0.0
        } else {
            // Penalty increases with time without nursing
            let excess_ticks = (self.ticks_since_nursed - 10) as f32;
            (excess_ticks * UNNURSED_HEALTH_LOSS).min(5.0) // Cap at 5 per tick
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_developmental_modifiers() {
        // Well-nourished development
        let mut good = DevelopmentalNutrition {
            prenatal_quality: 0.9,
            infant_quality: 0.85,
            child_quality: 0.8,
            ..Default::default()
        };
        good.finalize();
        assert!(good.stat_modifiers.max_health > 1.0);
        assert!(good.stat_modifiers.learning_rate > 1.0);

        // Poorly-nourished development
        let mut poor = DevelopmentalNutrition {
            prenatal_quality: 0.2,
            infant_quality: 0.3,
            child_quality: 0.25,
            ..Default::default()
        };
        poor.finalize();
        assert!(poor.stat_modifiers.max_health < 0.8);
        assert!(poor.stat_modifiers.fertility < 0.6);
    }

    #[test]
    fn test_nursing_state() {
        let mother_id = crate::core::dice::name();
        let mut nursing = NursingState::new(0, mother_id);

        assert!(nursing.needs_nursing(100));
        assert!(nursing.needs_nursing(499));
        assert!(!nursing.needs_nursing(500));

        assert!(nursing.is_caregiver(mother_id));
        assert!(!nursing.is_suffering());

        // Simulate lack of nursing
        for _ in 0..20 {
            nursing.tick_without_nursing();
        }
        assert!(nursing.is_suffering());
        assert!(nursing.health_penalty() > 0.0);

        // Nursing resets suffering
        nursing.nurse();
        assert!(!nursing.is_suffering());
        assert_eq!(nursing.health_penalty(), 0.0);
    }

    #[test]
    fn test_nutrition_updates() {
        let mut dev = DevelopmentalNutrition::default();

        // Good nursing
        for _ in 0..50 {
            dev.update_infant_nutrition(0.8, true);
        }
        assert!(dev.infant_quality > 0.85);

        // Poor nutrition without nursing
        let mut poor_dev = DevelopmentalNutrition::default();
        for _ in 0..50 {
            poor_dev.update_infant_nutrition(0.3, false);
        }
        assert!(poor_dev.infant_quality < 0.3);
    }
}
