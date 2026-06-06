// src/config/simulation.rs
//! General simulation parameters and tick configuration.

use serde::{Deserialize, Serialize};
use super::validation::{ConfigError, ConfigValidation};

/// Configuration for general simulation parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimulationConfig {
    /// Time and tick configuration
    pub time: TimeConfig,

    /// Population limits and behavior
    pub population: PopulationConfig,

    /// Resource and world parameters
    pub world: WorldSimConfig,

    /// Performance and optimization
    pub performance: PerformanceConfig,
}

/// Time and tick configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConfig {
    /// Ticks per in-game hour
    pub ticks_per_hour: u32,
    /// Hours per in-game day
    pub hours_per_day: u32,
    /// Days per in-game season
    pub days_per_season: u32,
    /// Seasons per in-game year
    pub seasons_per_year: u32,
}

impl TimeConfig {
    /// Calculate ticks per in-game day
    pub fn ticks_per_day(&self) -> u32 {
        self.ticks_per_hour * self.hours_per_day
    }

    /// Calculate ticks per in-game season
    pub fn ticks_per_season(&self) -> u32 {
        self.ticks_per_day() * self.days_per_season
    }

    /// Calculate ticks per in-game year
    pub fn ticks_per_year(&self) -> u32 {
        self.ticks_per_season() * self.seasons_per_year
    }
}

impl Default for TimeConfig {
    fn default() -> Self {
        Self {
            ticks_per_hour: 60,
            hours_per_day: 24,
            days_per_season: 30,
            seasons_per_year: 4,
        }
    }
}

/// Population limits and behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationConfig {
    /// Maximum agents in simulation
    pub max_agents: usize,
    /// Minimum agents before simulation ends (if applicable)
    pub min_agents: usize,
    /// Initial population size
    pub initial_population: usize,
    /// Agent interaction radius
    pub interaction_radius: f32,
    /// Social network maximum connections per agent
    pub max_relationships: usize,
}

impl Default for PopulationConfig {
    fn default() -> Self {
        Self {
            max_agents: 1000,
            min_agents: 0,
            initial_population: 10,
            interaction_radius: 15.0,
            max_relationships: 50,
        }
    }
}

/// World and resource simulation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSimConfig {
    /// Resource regeneration check interval (ticks)
    pub resource_regen_interval: u32,
    /// Weather change interval (ticks)
    pub weather_change_interval: u32,
    /// Day/night cycle affects behavior
    pub day_night_cycle_enabled: bool,
    /// Seasonal effects on resources
    pub seasonal_effects_enabled: bool,
}

impl Default for WorldSimConfig {
    fn default() -> Self {
        Self {
            resource_regen_interval: 100,
            weather_change_interval: 500,
            day_night_cycle_enabled: true,
            seasonal_effects_enabled: true,
        }
    }
}

/// Performance and optimization configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Batch size for parallel agent processing
    pub agent_batch_size: usize,
    /// Enable spatial partitioning for collision/interaction
    pub spatial_partitioning: bool,
    /// Cache pathfinding results for this many ticks
    pub pathfinding_cache_ticks: u32,
    /// Autosave interval (ticks, 0 = disabled)
    pub autosave_interval: u32,
    /// Maximum checkpoints to keep
    pub max_checkpoints: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            agent_batch_size: 100,
            spatial_partitioning: true,
            pathfinding_cache_ticks: 10,
            autosave_interval: 10000,
            max_checkpoints: 5,
        }
    }
}

impl ConfigValidation for SimulationConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate time configuration
        if self.time.ticks_per_hour == 0 {
            return Err(ConfigError::InvalidValue {
                field: "simulation.time.ticks_per_hour".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }
        if self.time.hours_per_day == 0 {
            return Err(ConfigError::InvalidValue {
                field: "simulation.time.hours_per_day".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }

        // Validate population
        if self.population.max_agents == 0 {
            return Err(ConfigError::InvalidValue {
                field: "simulation.population.max_agents".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }
        if self.population.initial_population > self.population.max_agents {
            return Err(ConfigError::InvalidOrder {
                field: "simulation.population".to_string(),
                message: "initial_population cannot exceed max_agents".to_string(),
            });
        }
        if self.population.interaction_radius <= 0.0 {
            return Err(ConfigError::NegativeValue {
                field: "simulation.population.interaction_radius".to_string(),
                value: self.population.interaction_radius,
            });
        }

        // Validate performance
        if self.performance.agent_batch_size == 0 {
            return Err(ConfigError::InvalidValue {
                field: "simulation.performance.agent_batch_size".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_simulation_config_is_valid() {
        let config = SimulationConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_time_calculations() {
        let config = TimeConfig::default();
        assert_eq!(config.ticks_per_day(), 60 * 24);
        assert_eq!(config.ticks_per_season(), 60 * 24 * 30);
        assert_eq!(config.ticks_per_year(), 60 * 24 * 30 * 4);
    }

    #[test]
    fn test_invalid_population() {
        let mut config = SimulationConfig::default();
        config.population.initial_population = 2000;
        config.population.max_agents = 1000; // Less than initial
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidOrder { .. })
        ));
    }
}
