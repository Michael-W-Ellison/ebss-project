// src/config/emotions.rs
//! Emotion system configuration for agent emotional states.

use serde::{Deserialize, Serialize};
use crate::core::EmotionType;
use super::validation::{ConfigError, ConfigValidation};

/// Configuration for the emotion system.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmotionsConfig {
    /// Natural decay rates per tick (emotions return to neutral)
    pub decay_rates: EmotionDecayRates,

    /// Intensity thresholds for emotion effects
    pub intensity_thresholds: EmotionIntensityThresholds,

    /// Modifiers for emotion interactions
    pub interaction_modifiers: EmotionInteractionModifiers,
}

/// Natural decay rates per tick for each emotion type.
/// Emotions naturally return to neutral (0.0) over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionDecayRates {
    /// Fear decays quickly (fight-or-flight response fades)
    pub fear: f32,
    /// Anger lingers longer
    pub anger: f32,
    /// Sadness decays slowly (grief takes time)
    pub sadness: f32,
    /// Happiness fades moderately
    pub happiness: f32,
    /// Curiosity persists (drives exploration)
    pub curiosity: f32,
}

impl EmotionDecayRates {
    /// Get decay rate for a specific emotion type
    pub fn get(&self, emotion_type: EmotionType) -> f32 {
        match emotion_type {
            EmotionType::Fear => self.fear,
            EmotionType::Anger => self.anger,
            EmotionType::Sadness => self.sadness,
            EmotionType::Happiness => self.happiness,
            EmotionType::Curiosity => self.curiosity,
        }
    }
}

impl Default for EmotionDecayRates {
    fn default() -> Self {
        Self {
            fear: 0.01,       // Fear decays quickly
            anger: 0.005,     // Anger lingers
            sadness: 0.003,   // Sadness decays slowly
            happiness: 0.008, // Happiness fades moderately
            curiosity: 0.002, // Curiosity persists
        }
    }
}

/// Thresholds for when emotions have significant effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionIntensityThresholds {
    /// Threshold for "mild" emotion effects
    pub mild: f32,
    /// Threshold for "moderate" emotion effects
    pub moderate: f32,
    /// Threshold for "strong" emotion effects
    pub strong: f32,
    /// Threshold for "extreme" emotion effects (may override behavior)
    pub extreme: f32,
}

impl Default for EmotionIntensityThresholds {
    fn default() -> Self {
        Self {
            mild: 0.2,
            moderate: 0.4,
            strong: 0.6,
            extreme: 0.8,
        }
    }
}

/// Modifiers for how emotions interact with each other and behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionInteractionModifiers {
    /// How much fear reduces when safety drive is satisfied
    pub fear_safety_reduction: f32,
    /// How much happiness increases from positive social interaction
    pub happiness_social_boost: f32,
    /// How much anger increases combat effectiveness
    pub anger_combat_boost: f32,
    /// How much sadness reduces work efficiency
    pub sadness_efficiency_penalty: f32,
    /// How much curiosity boosts exploration success
    pub curiosity_exploration_boost: f32,
    /// Grief intensity multiplier for close relationships
    pub grief_relationship_multiplier: f32,
}

impl Default for EmotionInteractionModifiers {
    fn default() -> Self {
        Self {
            fear_safety_reduction: 0.3,
            happiness_social_boost: 0.15,
            anger_combat_boost: 0.2,
            sadness_efficiency_penalty: 0.25,
            curiosity_exploration_boost: 0.3,
            grief_relationship_multiplier: 1.5,
        }
    }
}

impl ConfigValidation for EmotionsConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate decay rates are positive and reasonable
        let decay_fields = [
            ("fear", self.decay_rates.fear),
            ("anger", self.decay_rates.anger),
            ("sadness", self.decay_rates.sadness),
            ("happiness", self.decay_rates.happiness),
            ("curiosity", self.decay_rates.curiosity),
        ];

        for (name, value) in decay_fields {
            if value < 0.0 {
                return Err(ConfigError::NegativeValue {
                    field: format!("emotions.decay_rates.{}", name),
                    value,
                });
            }
            if value > 0.5 {
                return Err(ConfigError::OutOfRange {
                    field: format!("emotions.decay_rates.{}", name),
                    value,
                    min: 0.0,
                    max: 0.5,
                });
            }
        }

        // Validate intensity thresholds are ordered correctly
        let thresholds = &self.intensity_thresholds;
        if thresholds.mild >= thresholds.moderate {
            return Err(ConfigError::InvalidOrder {
                field: "emotions.intensity_thresholds".to_string(),
                message: "mild must be less than moderate".to_string(),
            });
        }
        if thresholds.moderate >= thresholds.strong {
            return Err(ConfigError::InvalidOrder {
                field: "emotions.intensity_thresholds".to_string(),
                message: "moderate must be less than strong".to_string(),
            });
        }
        if thresholds.strong >= thresholds.extreme {
            return Err(ConfigError::InvalidOrder {
                field: "emotions.intensity_thresholds".to_string(),
                message: "strong must be less than extreme".to_string(),
            });
        }

        // Validate all thresholds are in 0.0-1.0 range
        for (name, value) in [
            ("mild", thresholds.mild),
            ("moderate", thresholds.moderate),
            ("strong", thresholds.strong),
            ("extreme", thresholds.extreme),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(ConfigError::OutOfRange {
                    field: format!("emotions.intensity_thresholds.{}", name),
                    value,
                    min: 0.0,
                    max: 1.0,
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_emotions_config_is_valid() {
        let config = EmotionsConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_negative_decay_rate() {
        let mut config = EmotionsConfig::default();
        config.decay_rates.fear = -0.01;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::NegativeValue { .. })
        ));
    }

    #[test]
    fn test_invalid_threshold_order() {
        let mut config = EmotionsConfig::default();
        config.intensity_thresholds.mild = 0.5;
        config.intensity_thresholds.moderate = 0.3; // Wrong order
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidOrder { .. })
        ));
    }

    #[test]
    fn test_get_decay_rate() {
        let config = EmotionsConfig::default();
        assert_eq!(config.decay_rates.get(EmotionType::Fear), 0.01);
        assert_eq!(config.decay_rates.get(EmotionType::Sadness), 0.003);
    }
}
