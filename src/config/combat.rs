// src/config/combat.rs
//! Combat system configuration for damage, armor, and weapon balance.

use serde::{Deserialize, Serialize};
use super::validation::{ConfigError, ConfigValidation};

/// Configuration for the combat system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatConfig {
    /// Damage calculation parameters
    pub damage: DamageConfig,

    /// Defense and armor parameters
    pub defense: DefenseConfig,

    /// Weapon range and effectiveness
    pub weapons: WeaponConfig,

    /// Combat experience and skill progression
    pub experience: CombatExperienceConfig,

    /// Injury system thresholds
    pub injuries: InjuryConfig,
}

/// Damage calculation parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageConfig {
    /// Base unarmed damage
    pub base_unarmed: f32,
    /// Damage variance range (0.8 to 1.2 = +/- 20%)
    pub variance_min: f32,
    pub variance_max: f32,
    /// Critical hit multiplier
    pub critical_multiplier: f32,
    /// Critical hit chance (0.0 - 1.0)
    pub critical_chance: f32,
    /// Strength scaling factor
    pub strength_scaling: f32,
}

impl Default for DamageConfig {
    fn default() -> Self {
        Self {
            base_unarmed: 5.0,
            variance_min: 0.8,
            variance_max: 1.2,
            critical_multiplier: 2.0,
            critical_chance: 0.1,
            strength_scaling: 0.5,
        }
    }
}

/// Defense and armor parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenseConfig {
    /// Maximum damage reduction from armor (0.0 - 1.0)
    pub max_damage_reduction: f32,
    /// Armor degradation per hit
    pub armor_degradation_per_hit: f32,
    /// Block chance bonus per shield level
    pub shield_block_bonus: f32,
    /// Dodge chance base value
    pub base_dodge_chance: f32,
}

impl Default for DefenseConfig {
    fn default() -> Self {
        Self {
            max_damage_reduction: 0.75,
            armor_degradation_per_hit: 0.01,
            shield_block_bonus: 0.15,
            base_dodge_chance: 0.05,
        }
    }
}

/// Weapon configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponConfig {
    /// Default melee range
    pub melee_range: f32,
    /// Default bow range
    pub bow_range: f32,
    /// Default crossbow range
    pub crossbow_range: f32,
    /// Weapon durability loss per use
    pub durability_loss_per_use: f32,
}

impl Default for WeaponConfig {
    fn default() -> Self {
        Self {
            melee_range: 1.5,
            bow_range: 15.0,
            crossbow_range: 20.0,
            durability_loss_per_use: 0.02,
        }
    }
}

/// Combat experience configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatExperienceConfig {
    /// XP gained per successful hit
    pub xp_per_hit: u32,
    /// XP gained per kill
    pub xp_per_kill: u32,
    /// XP bonus for fighting stronger opponents
    pub difficulty_xp_multiplier: f32,
    /// Combat skill effect on damage (per skill level)
    pub skill_damage_bonus: f32,
}

impl Default for CombatExperienceConfig {
    fn default() -> Self {
        Self {
            xp_per_hit: 2,
            xp_per_kill: 5,
            difficulty_xp_multiplier: 1.5,
            skill_damage_bonus: 0.05,
        }
    }
}

/// Injury system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjuryConfig {
    /// Damage threshold for minor injury
    pub minor_threshold: f32,
    /// Damage threshold for moderate injury
    pub moderate_threshold: f32,
    /// Damage threshold for severe injury
    pub severe_threshold: f32,
    /// Bleeding damage per tick
    pub bleed_damage_per_tick: f32,
    /// Infection chance for untreated wounds
    pub infection_chance: f32,
}

impl Default for InjuryConfig {
    fn default() -> Self {
        Self {
            minor_threshold: 10.0,
            moderate_threshold: 15.0,
            severe_threshold: 30.0,
            bleed_damage_per_tick: 0.5,
            infection_chance: 0.1,
        }
    }
}

impl Default for CombatConfig {
    fn default() -> Self {
        Self {
            damage: DamageConfig::default(),
            defense: DefenseConfig::default(),
            weapons: WeaponConfig::default(),
            experience: CombatExperienceConfig::default(),
            injuries: InjuryConfig::default(),
        }
    }
}

impl ConfigValidation for CombatConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate damage variance
        if self.damage.variance_min > self.damage.variance_max {
            return Err(ConfigError::InvalidOrder {
                field: "combat.damage.variance".to_string(),
                message: "variance_min must be less than variance_max".to_string(),
            });
        }
        if self.damage.variance_min < 0.0 {
            return Err(ConfigError::NegativeValue {
                field: "combat.damage.variance_min".to_string(),
                value: self.damage.variance_min,
            });
        }

        // Validate critical hit parameters
        if !(0.0..=1.0).contains(&self.damage.critical_chance) {
            return Err(ConfigError::OutOfRange {
                field: "combat.damage.critical_chance".to_string(),
                value: self.damage.critical_chance,
                min: 0.0,
                max: 1.0,
            });
        }

        // Validate defense parameters
        if !(0.0..=1.0).contains(&self.defense.max_damage_reduction) {
            return Err(ConfigError::OutOfRange {
                field: "combat.defense.max_damage_reduction".to_string(),
                value: self.defense.max_damage_reduction,
                min: 0.0,
                max: 1.0,
            });
        }

        // Validate injury thresholds are ordered
        if self.injuries.minor_threshold >= self.injuries.moderate_threshold {
            return Err(ConfigError::InvalidOrder {
                field: "combat.injuries".to_string(),
                message: "minor_threshold must be less than moderate_threshold".to_string(),
            });
        }
        if self.injuries.moderate_threshold >= self.injuries.severe_threshold {
            return Err(ConfigError::InvalidOrder {
                field: "combat.injuries".to_string(),
                message: "moderate_threshold must be less than severe_threshold".to_string(),
            });
        }

        // Validate weapon ranges are positive
        if self.weapons.melee_range <= 0.0 {
            return Err(ConfigError::NegativeValue {
                field: "combat.weapons.melee_range".to_string(),
                value: self.weapons.melee_range,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_combat_config_is_valid() {
        let config = CombatConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_damage_variance() {
        let mut config = CombatConfig::default();
        config.damage.variance_min = 1.5;
        config.damage.variance_max = 1.2; // Wrong order
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidOrder { .. })
        ));
    }

    #[test]
    fn test_invalid_critical_chance() {
        let mut config = CombatConfig::default();
        config.damage.critical_chance = 1.5; // Invalid
        assert!(matches!(
            config.validate(),
            Err(ConfigError::OutOfRange { .. })
        ));
    }
}
