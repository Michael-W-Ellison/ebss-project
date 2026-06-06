// src/config/drives.rs
//! Drive system configuration for agent motivation and behavior.

use serde::{Deserialize, Serialize};
use crate::core::DriveType;
use super::validation::{ConfigError, ConfigValidation};

/// Configuration for the drive system that motivates agent behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrivesConfig {
    /// Threshold values at which drives become "urgent" (0.0 - 1.0)
    pub thresholds: DriveThresholds,

    /// Base accumulation rates per tick for each drive
    pub accumulation_rates: DriveAccumulationRates,

    /// Satisfaction amounts when drives are fulfilled
    pub satisfaction_amounts: DriveSatisfactionAmounts,

    /// Weight multipliers for drive urgency calculations
    pub urgency_weights: DriveUrgencyWeights,
}

/// Threshold values at which drives become urgent and require attention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveThresholds {
    pub hunger: f32,
    pub thirst: f32,
    pub rest: f32,
    pub shelter: f32,
    pub safety: f32,
    pub preparedness: f32,
    pub industry: f32,
    pub sustenance: f32,
    pub curiosity: f32,
    pub social: f32,
    pub reproduction: f32,
    pub luxury: f32,
    pub utility: f32,
    pub construction: f32,
}

impl DriveThresholds {
    /// Get threshold for a specific drive type
    pub fn get(&self, drive_type: DriveType) -> f32 {
        match drive_type {
            DriveType::Hunger => self.hunger,
            DriveType::Thirst => self.thirst,
            DriveType::Rest => self.rest,
            DriveType::Shelter => self.shelter,
            DriveType::Safety => self.safety,
            DriveType::Preparedness => self.preparedness,
            DriveType::Industry => self.industry,
            DriveType::Sustenance => self.sustenance,
            DriveType::Curiosity => self.curiosity,
            DriveType::Social => self.social,
            DriveType::Reproduction => self.reproduction,
            DriveType::Luxury => self.luxury,
            DriveType::Utility => self.utility,
            DriveType::Construction => self.construction,
        }
    }
}

impl Default for DriveThresholds {
    fn default() -> Self {
        Self {
            hunger: 0.7,
            thirst: 0.75,
            rest: 0.6,
            shelter: 0.5,
            safety: 0.8,
            preparedness: 0.4,
            industry: 0.3,
            sustenance: 0.3,
            curiosity: 0.2,
            social: 0.5,
            reproduction: 0.6,
            luxury: 0.1,
            utility: 0.4,
            construction: 0.3,
        }
    }
}

/// Base accumulation rates per tick for each drive type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveAccumulationRates {
    pub hunger: f32,
    pub thirst: f32,
    pub rest: f32,
    pub shelter: f32,
    pub safety: f32,
    pub preparedness: f32,
    pub industry: f32,
    pub sustenance: f32,
    pub curiosity: f32,
    pub social: f32,
    pub reproduction: f32,
    pub luxury: f32,
    pub utility: f32,
    pub construction: f32,
}

impl DriveAccumulationRates {
    /// Get accumulation rate for a specific drive type
    pub fn get(&self, drive_type: DriveType) -> f32 {
        match drive_type {
            DriveType::Hunger => self.hunger,
            DriveType::Thirst => self.thirst,
            DriveType::Rest => self.rest,
            DriveType::Shelter => self.shelter,
            DriveType::Safety => self.safety,
            DriveType::Preparedness => self.preparedness,
            DriveType::Industry => self.industry,
            DriveType::Sustenance => self.sustenance,
            DriveType::Curiosity => self.curiosity,
            DriveType::Social => self.social,
            DriveType::Reproduction => self.reproduction,
            DriveType::Luxury => self.luxury,
            DriveType::Utility => self.utility,
            DriveType::Construction => self.construction,
        }
    }
}

impl Default for DriveAccumulationRates {
    fn default() -> Self {
        Self {
            hunger: 0.01,
            thirst: 0.012,    // Slightly faster than hunger
            rest: 0.008,
            shelter: 0.005,
            safety: 0.02,     // Spikes with threats
            preparedness: 0.002,
            industry: 0.003,
            sustenance: 0.003,
            curiosity: 0.004,
            social: 0.006,
            reproduction: 0.001,
            luxury: 0.001,
            utility: 0.002,
            construction: 0.002,
        }
    }
}

/// Satisfaction amounts when drives are fulfilled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveSatisfactionAmounts {
    /// How much hunger decreases when eating a standard meal
    pub hunger_per_meal: f32,
    /// How much thirst decreases when drinking
    pub thirst_per_drink: f32,
    /// How much rest decreases per tick of sleep
    pub rest_per_sleep_tick: f32,
    /// Shelter satisfaction when inside a building
    pub shelter_inside_building: f32,
    /// Safety satisfaction from being armed
    pub safety_with_weapon: f32,
    /// Social satisfaction from positive interaction
    pub social_per_interaction: f32,
}

impl Default for DriveSatisfactionAmounts {
    fn default() -> Self {
        Self {
            hunger_per_meal: 0.5,
            thirst_per_drink: 0.6,
            rest_per_sleep_tick: 0.02,
            shelter_inside_building: 0.3,
            safety_with_weapon: 0.2,
            social_per_interaction: 0.15,
        }
    }
}

/// Weight multipliers for drive urgency calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveUrgencyWeights {
    /// Survival drives (hunger, thirst, rest) weight multiplier
    pub survival: f32,
    /// Safety drives weight multiplier
    pub safety: f32,
    /// Social drives weight multiplier
    pub social: f32,
    /// Self-actualization drives (curiosity, luxury) weight multiplier
    pub self_actualization: f32,
}

impl Default for DriveUrgencyWeights {
    fn default() -> Self {
        Self {
            survival: 2.0,
            safety: 1.5,
            social: 1.0,
            self_actualization: 0.5,
        }
    }
}

impl Default for DrivesConfig {
    fn default() -> Self {
        Self {
            thresholds: DriveThresholds::default(),
            accumulation_rates: DriveAccumulationRates::default(),
            satisfaction_amounts: DriveSatisfactionAmounts::default(),
            urgency_weights: DriveUrgencyWeights::default(),
        }
    }
}

impl ConfigValidation for DrivesConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate thresholds are in valid range
        let threshold_fields = [
            ("hunger", self.thresholds.hunger),
            ("thirst", self.thresholds.thirst),
            ("rest", self.thresholds.rest),
            ("shelter", self.thresholds.shelter),
            ("safety", self.thresholds.safety),
            ("preparedness", self.thresholds.preparedness),
            ("industry", self.thresholds.industry),
            ("sustenance", self.thresholds.sustenance),
            ("curiosity", self.thresholds.curiosity),
            ("social", self.thresholds.social),
            ("reproduction", self.thresholds.reproduction),
            ("luxury", self.thresholds.luxury),
            ("utility", self.thresholds.utility),
            ("construction", self.thresholds.construction),
        ];

        for (name, value) in threshold_fields {
            if !(0.0..=1.0).contains(&value) {
                return Err(ConfigError::OutOfRange {
                    field: format!("drives.thresholds.{}", name),
                    value,
                    min: 0.0,
                    max: 1.0,
                });
            }
        }

        // Validate accumulation rates are positive
        let rate_fields = [
            ("hunger", self.accumulation_rates.hunger),
            ("thirst", self.accumulation_rates.thirst),
            ("rest", self.accumulation_rates.rest),
            ("shelter", self.accumulation_rates.shelter),
            ("safety", self.accumulation_rates.safety),
            ("preparedness", self.accumulation_rates.preparedness),
            ("industry", self.accumulation_rates.industry),
            ("sustenance", self.accumulation_rates.sustenance),
            ("curiosity", self.accumulation_rates.curiosity),
            ("social", self.accumulation_rates.social),
            ("reproduction", self.accumulation_rates.reproduction),
            ("luxury", self.accumulation_rates.luxury),
            ("utility", self.accumulation_rates.utility),
            ("construction", self.accumulation_rates.construction),
        ];

        for (name, value) in rate_fields {
            if value < 0.0 {
                return Err(ConfigError::NegativeValue {
                    field: format!("drives.accumulation_rates.{}", name),
                    value,
                });
            }
            if value > 1.0 {
                return Err(ConfigError::OutOfRange {
                    field: format!("drives.accumulation_rates.{}", name),
                    value,
                    min: 0.0,
                    max: 1.0,
                });
            }
        }

        // Validate urgency weights are positive
        if self.urgency_weights.survival <= 0.0 {
            return Err(ConfigError::NegativeValue {
                field: "drives.urgency_weights.survival".to_string(),
                value: self.urgency_weights.survival,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_drives_config_is_valid() {
        let config = DrivesConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_threshold_out_of_range() {
        let mut config = DrivesConfig::default();
        config.thresholds.hunger = 1.5; // Invalid
        assert!(matches!(
            config.validate(),
            Err(ConfigError::OutOfRange { field, .. }) if field.contains("hunger")
        ));
    }

    #[test]
    fn test_negative_accumulation_rate() {
        let mut config = DrivesConfig::default();
        config.accumulation_rates.thirst = -0.01; // Invalid
        assert!(matches!(
            config.validate(),
            Err(ConfigError::NegativeValue { field, .. }) if field.contains("thirst")
        ));
    }

    #[test]
    fn test_get_threshold() {
        let config = DrivesConfig::default();
        assert_eq!(config.thresholds.get(DriveType::Hunger), 0.7);
        assert_eq!(config.thresholds.get(DriveType::Thirst), 0.75);
    }

    #[test]
    fn test_get_accumulation_rate() {
        let config = DrivesConfig::default();
        assert_eq!(config.accumulation_rates.get(DriveType::Hunger), 0.01);
        assert_eq!(config.accumulation_rates.get(DriveType::Thirst), 0.012);
    }
}
