// src/config/mod.rs
//! Centralized configuration system for game balance and simulation parameters.
//!
//! This module provides a type-safe, hierarchical configuration system that can be
//! loaded from TOML files or constructed programmatically with sensible defaults.

pub mod combat;
pub mod drives;
pub mod emotions;
pub mod learning;
pub mod simulation;
pub mod survival;
pub mod validation;

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

pub use combat::CombatConfig;
pub use drives::DrivesConfig;
pub use emotions::EmotionsConfig;
pub use learning::LearningConfig;
pub use simulation::SimulationConfig;
pub use survival::SurvivalConfig;
pub use validation::{ConfigError, ConfigValidation};

/// Global configuration instance
static GLOBAL_CONFIG: OnceLock<GameConfig> = OnceLock::new();

/// Master configuration struct containing all game balance parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    /// Drive system configuration (hunger, thirst, rest, etc.)
    pub drives: DrivesConfig,

    /// Emotion system configuration (fear, anger, happiness decay rates)
    pub emotions: EmotionsConfig,

    /// Survival mechanics (starvation, dehydration timings)
    pub survival: SurvivalConfig,

    /// Combat balance (damage, armor, ranges)
    pub combat: CombatConfig,

    /// Learning and skill progression
    pub learning: LearningConfig,

    /// General simulation parameters
    pub simulation: SimulationConfig,
}

impl GameConfig {
    /// Load configuration from a TOML file
    pub fn from_toml(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        let config: Self = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_toml(&self, path: &Path) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeError(e.to_string()))?;

        std::fs::write(path, content)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Validate all configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.drives.validate()?;
        self.emotions.validate()?;
        self.survival.validate()?;
        self.combat.validate()?;
        self.learning.validate()?;
        self.simulation.validate()?;
        Ok(())
    }

    /// Initialize the global configuration with defaults
    pub fn init_global() -> &'static Self {
        GLOBAL_CONFIG.get_or_init(Self::default)
    }

    /// Initialize the global configuration from a file
    pub fn init_global_from_file(path: &Path) -> Result<&'static Self, ConfigError> {
        let config = Self::from_toml(path)?;
        Ok(GLOBAL_CONFIG.get_or_init(|| config))
    }

    /// Get the global configuration (panics if not initialized)
    pub fn global() -> &'static Self {
        GLOBAL_CONFIG.get().expect("GameConfig not initialized. Call init_global() first.")
    }

    /// Try to get the global configuration
    pub fn try_global() -> Option<&'static Self> {
        GLOBAL_CONFIG.get()
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            drives: DrivesConfig::default(),
            emotions: EmotionsConfig::default(),
            survival: SurvivalConfig::default(),
            combat: CombatConfig::default(),
            learning: LearningConfig::default(),
            simulation: SimulationConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config_is_valid() {
        let config = GameConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = GameConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: GameConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn test_config_file_roundtrip() {
        let config = GameConfig::default();
        let mut file = NamedTempFile::new().unwrap();

        config.to_toml(file.path()).unwrap();
        let loaded = GameConfig::from_toml(file.path()).unwrap();

        assert!(loaded.validate().is_ok());
    }
}
