// src/config/learning.rs
//! Learning and skill progression configuration.

use serde::{Deserialize, Serialize};
use super::validation::{ConfigError, ConfigValidation};

/// Configuration for learning and skill progression systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// General skill progression parameters
    pub skills: SkillProgressionConfig,

    /// Observational learning (learning by watching others)
    pub observational: ObservationalLearningConfig,

    /// Recipe and crafting discovery
    pub discovery: DiscoveryConfig,

    /// Memory and knowledge retention
    pub memory: MemoryConfig,
}

/// Skill progression configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProgressionConfig {
    /// Base XP required to level up (multiplied by level)
    pub base_xp_per_level: u32,
    /// XP scaling factor per level
    pub xp_scaling_factor: f32,
    /// Maximum skill level
    pub max_level: u32,
    /// Skill decay rate when not used (per tick)
    pub decay_rate: f32,
    /// Minimum skill level (skills don't decay below this)
    pub minimum_level: i32,
}

impl Default for SkillProgressionConfig {
    fn default() -> Self {
        Self {
            base_xp_per_level: 100,
            xp_scaling_factor: 1.5,
            max_level: 100,
            decay_rate: 0.0001,
            minimum_level: -10,
        }
    }
}

/// Observational learning configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationalLearningConfig {
    /// Minimum observations required to learn a skill
    pub min_observations: u32,
    /// Learning efficiency multiplier
    pub efficiency_multiplier: f32,
    /// Maximum distance to observe another agent
    pub max_observation_distance: f32,
    /// Memory decay for observations (per tick)
    pub observation_decay: f32,
    /// Bonus for observing a master (high skill agent)
    pub master_observation_bonus: f32,
}

impl Default for ObservationalLearningConfig {
    fn default() -> Self {
        Self {
            min_observations: 3,
            efficiency_multiplier: 0.5,
            max_observation_distance: 10.0,
            observation_decay: 0.001,
            master_observation_bonus: 0.3,
        }
    }
}

/// Recipe and technology discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Base chance to discover a recipe when experimenting
    pub base_discovery_chance: f32,
    /// Bonus discovery chance per relevant skill level
    pub skill_discovery_bonus: f32,
    /// Chance to remember a discovered recipe permanently
    pub permanent_memory_chance: f32,
    /// Ticks required to attempt a discovery
    pub discovery_attempt_time: u32,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            base_discovery_chance: 0.1,
            skill_discovery_bonus: 0.02,
            permanent_memory_chance: 0.8,
            discovery_attempt_time: 60,
        }
    }
}

/// Memory and knowledge retention configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Working memory capacity (number of active tasks)
    pub working_memory_capacity: usize,
    /// Episodic memory capacity (number of remembered events)
    pub episodic_memory_capacity: usize,
    /// Memory consolidation threshold (strength required to persist)
    pub consolidation_threshold: f32,
    /// Memory decay rate for unconsolidated memories
    pub decay_rate: f32,
    /// Knowledge sharing effectiveness with others
    pub teaching_effectiveness: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            working_memory_capacity: 5,
            episodic_memory_capacity: 100,
            consolidation_threshold: 0.5,
            decay_rate: 0.01,
            teaching_effectiveness: 0.7,
        }
    }
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            skills: SkillProgressionConfig::default(),
            observational: ObservationalLearningConfig::default(),
            discovery: DiscoveryConfig::default(),
            memory: MemoryConfig::default(),
        }
    }
}

impl ConfigValidation for LearningConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate skill progression
        if self.skills.base_xp_per_level == 0 {
            return Err(ConfigError::InvalidValue {
                field: "learning.skills.base_xp_per_level".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }
        if self.skills.xp_scaling_factor < 1.0 {
            return Err(ConfigError::OutOfRange {
                field: "learning.skills.xp_scaling_factor".to_string(),
                value: self.skills.xp_scaling_factor,
                min: 1.0,
                max: f32::MAX,
            });
        }
        if self.skills.max_level == 0 {
            return Err(ConfigError::InvalidValue {
                field: "learning.skills.max_level".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }

        // Validate observational learning
        if self.observational.min_observations == 0 {
            return Err(ConfigError::InvalidValue {
                field: "learning.observational.min_observations".to_string(),
                message: "must be greater than 0".to_string(),
            });
        }
        if self.observational.max_observation_distance <= 0.0 {
            return Err(ConfigError::NegativeValue {
                field: "learning.observational.max_observation_distance".to_string(),
                value: self.observational.max_observation_distance,
            });
        }

        // Validate discovery chances
        if !(0.0..=1.0).contains(&self.discovery.base_discovery_chance) {
            return Err(ConfigError::OutOfRange {
                field: "learning.discovery.base_discovery_chance".to_string(),
                value: self.discovery.base_discovery_chance,
                min: 0.0,
                max: 1.0,
            });
        }
        if !(0.0..=1.0).contains(&self.discovery.permanent_memory_chance) {
            return Err(ConfigError::OutOfRange {
                field: "learning.discovery.permanent_memory_chance".to_string(),
                value: self.discovery.permanent_memory_chance,
                min: 0.0,
                max: 1.0,
            });
        }

        // Validate memory capacity
        if self.memory.working_memory_capacity == 0 {
            return Err(ConfigError::InvalidValue {
                field: "learning.memory.working_memory_capacity".to_string(),
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
    fn test_default_learning_config_is_valid() {
        let config = LearningConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_zero_xp_per_level() {
        let mut config = LearningConfig::default();
        config.skills.base_xp_per_level = 0;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidValue { .. })
        ));
    }

    #[test]
    fn test_invalid_discovery_chance() {
        let mut config = LearningConfig::default();
        config.discovery.base_discovery_chance = 1.5;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::OutOfRange { .. })
        ));
    }
}
