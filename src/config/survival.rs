// src/config/survival.rs
//! Survival mechanics configuration for starvation, dehydration, and environmental hazards.

use serde::{Deserialize, Serialize};
use super::validation::{ConfigError, ConfigValidation};

/// Configuration for survival mechanics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SurvivalConfig {
    /// Starvation mechanics (food deprivation)
    pub starvation: StarvationConfig,

    /// Dehydration mechanics (water deprivation)
    pub dehydration: DehydrationConfig,

    /// Energy mechanics
    pub energy: EnergyConfig,

    /// Environmental hazard thresholds
    pub environment: EnvironmentHazardConfig,
}

/// Starvation timing and damage configuration.
/// Values are in ticks (60 ticks = 1 hour in simulation time).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarvationConfig {
    /// Ticks without food before energy depletion accelerates (24 hours = 1440 ticks)
    pub energy_acceleration_threshold: u32,
    /// Energy depletion multiplier after threshold
    pub energy_acceleration_multiplier: f32,

    /// Ticks without food before health starts decreasing (3 days = 4320 ticks)
    pub health_damage_threshold: u32,
    /// Health damage per tick after threshold
    pub health_damage_per_tick: f32,

    /// Ticks without food before rapid health loss (7 days = 10080 ticks)
    pub critical_threshold: u32,
    /// Severe health damage per tick at critical stage
    pub critical_damage_per_tick: f32,
}

impl Default for StarvationConfig {
    fn default() -> Self {
        Self {
            energy_acceleration_threshold: 1440,  // 24 hours
            energy_acceleration_multiplier: 2.0,

            health_damage_threshold: 4320,        // 3 days
            health_damage_per_tick: 0.1,

            critical_threshold: 10080,            // 7 days
            critical_damage_per_tick: 1.0,
        }
    }
}

/// Dehydration timing and damage configuration.
/// Dehydration progresses faster than starvation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DehydrationConfig {
    /// Ticks without water before energy depletion accelerates (12 hours = 720 ticks)
    pub energy_acceleration_threshold: u32,
    /// Additional energy depletion multiplier after threshold
    pub energy_acceleration_multiplier: f32,

    /// Ticks without water before health starts decreasing (1.5 days = 2160 ticks)
    pub health_damage_threshold: u32,
    /// Health damage per tick after threshold
    pub health_damage_per_tick: f32,

    /// Ticks without water before rapid health loss (3 days = 4320 ticks)
    pub critical_threshold: u32,
    /// Severe health damage per tick at critical stage
    pub critical_damage_per_tick: f32,
}

impl Default for DehydrationConfig {
    fn default() -> Self {
        Self {
            energy_acceleration_threshold: 720,   // 12 hours
            energy_acceleration_multiplier: 1.5,

            health_damage_threshold: 2160,        // 1.5 days
            health_damage_per_tick: 0.15,

            critical_threshold: 4320,             // 3 days
            critical_damage_per_tick: 1.5,
        }
    }
}

/// Energy mechanics configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyConfig {
    /// Base energy loss per tick (normal metabolism)
    pub base_loss_per_tick: f32,
    /// Health damage per tick when energy is depleted
    pub depleted_health_damage: f32,
    /// Energy recovery per tick while sleeping
    pub sleep_recovery_per_tick: f32,
    /// Energy cost multiplier for heavy labor
    pub heavy_labor_multiplier: f32,
    /// Energy cost multiplier for combat
    pub combat_multiplier: f32,
}

impl Default for EnergyConfig {
    fn default() -> Self {
        Self {
            base_loss_per_tick: 0.05,
            depleted_health_damage: 0.05,
            sleep_recovery_per_tick: 0.1,
            heavy_labor_multiplier: 2.0,
            combat_multiplier: 3.0,
        }
    }
}

/// Environmental hazard damage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentHazardConfig {
    /// Cold damage range (min, max) per tick without insulation
    pub cold_damage_range: (f32, f32),
    /// Heat damage range (min, max) per tick without heat resistance
    pub heat_damage_range: (f32, f32),
    /// Fall damage multiplier per height unit
    pub fall_damage_per_height: (f32, f32),
    /// Threshold height for injury severity
    pub moderate_fall_damage_threshold: f32,
    /// Threshold for severe injury
    pub severe_fall_damage_threshold: f32,
}

impl Default for EnvironmentHazardConfig {
    fn default() -> Self {
        Self {
            cold_damage_range: (1.0, 5.0),
            heat_damage_range: (2.0, 8.0),
            fall_damage_per_height: (3.0, 8.0),
            moderate_fall_damage_threshold: 12.0,
            severe_fall_damage_threshold: 25.0,
        }
    }
}

impl ConfigValidation for SurvivalConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate starvation thresholds are ordered correctly
        if self.starvation.energy_acceleration_threshold >= self.starvation.health_damage_threshold {
            return Err(ConfigError::InvalidOrder {
                field: "survival.starvation".to_string(),
                message: "energy_acceleration_threshold must be less than health_damage_threshold".to_string(),
            });
        }
        if self.starvation.health_damage_threshold >= self.starvation.critical_threshold {
            return Err(ConfigError::InvalidOrder {
                field: "survival.starvation".to_string(),
                message: "health_damage_threshold must be less than critical_threshold".to_string(),
            });
        }

        // Validate dehydration thresholds are ordered correctly
        if self.dehydration.energy_acceleration_threshold >= self.dehydration.health_damage_threshold {
            return Err(ConfigError::InvalidOrder {
                field: "survival.dehydration".to_string(),
                message: "energy_acceleration_threshold must be less than health_damage_threshold".to_string(),
            });
        }
        if self.dehydration.health_damage_threshold >= self.dehydration.critical_threshold {
            return Err(ConfigError::InvalidOrder {
                field: "survival.dehydration".to_string(),
                message: "health_damage_threshold must be less than critical_threshold".to_string(),
            });
        }

        // Validate dehydration is faster than starvation (realistic)
        if self.dehydration.critical_threshold > self.starvation.critical_threshold {
            return Err(ConfigError::InvalidOrder {
                field: "survival".to_string(),
                message: "dehydration critical_threshold should not exceed starvation critical_threshold".to_string(),
            });
        }

        // Validate energy values are positive
        if self.energy.base_loss_per_tick <= 0.0 {
            return Err(ConfigError::NegativeValue {
                field: "survival.energy.base_loss_per_tick".to_string(),
                value: self.energy.base_loss_per_tick,
            });
        }

        // Validate damage ranges
        let env = &self.environment;
        if env.cold_damage_range.0 > env.cold_damage_range.1 {
            return Err(ConfigError::InvalidOrder {
                field: "survival.environment.cold_damage_range".to_string(),
                message: "min must be less than or equal to max".to_string(),
            });
        }
        if env.heat_damage_range.0 > env.heat_damage_range.1 {
            return Err(ConfigError::InvalidOrder {
                field: "survival.environment.heat_damage_range".to_string(),
                message: "min must be less than or equal to max".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_survival_config_is_valid() {
        let config = SurvivalConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_starvation_order() {
        let mut config = SurvivalConfig::default();
        config.starvation.energy_acceleration_threshold = 5000;
        config.starvation.health_damage_threshold = 4320; // Wrong order
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidOrder { .. })
        ));
    }

    #[test]
    fn test_dehydration_faster_than_starvation() {
        let config = SurvivalConfig::default();
        assert!(config.dehydration.critical_threshold < config.starvation.critical_threshold);
    }
}
