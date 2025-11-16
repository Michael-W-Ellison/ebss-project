// src/world/combat.rs
//! Combat system for EBSS
//!
//! Handles combat between agents, animals, and agents vs animals.
//! Includes damage calculation, armor mitigation, weapon bonuses, and death handling.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Result of a combat action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatResult {
    pub attacker_id: Uuid,
    pub defender_id: Uuid,
    pub damage_dealt: f32,
    pub damage_mitigated: f32,
    pub defender_killed: bool,
    pub weapon_used: Option<String>,
    pub critical_hit: bool,
}

/// Combat action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatAction {
    /// Standard melee attack
    MeleeAttack,
    /// Ranged attack (if ranged weapon equipped)
    RangedAttack,
    /// Heavy attack (more damage, slower)
    HeavyAttack,
    /// Quick attack (less damage, faster)
    QuickAttack,
    /// Defensive stance (reduced damage taken)
    Defend,
}

/// Combat stats for an entity
#[derive(Debug, Clone)]
pub struct CombatStats {
    /// Base damage before modifiers
    pub base_damage: f32,
    /// Weapon damage bonus
    pub weapon_damage: f32,
    /// Armor rating (0.0 - 1.0, reduces damage)
    pub armor_rating: f32,
    /// Attack speed multiplier
    pub attack_speed: f32,
    /// Critical hit chance (0.0 - 1.0)
    pub crit_chance: f32,
    /// Critical hit damage multiplier
    pub crit_multiplier: f32,
    /// Mounted combat bonus
    pub mounted_bonus: f32,
}

impl Default for CombatStats {
    fn default() -> Self {
        Self {
            base_damage: 5.0,
            weapon_damage: 0.0,
            armor_rating: 0.0,
            attack_speed: 1.0,
            crit_chance: 0.05,
            crit_multiplier: 1.5,
            mounted_bonus: 0.0,
        }
    }
}

impl CombatStats {
    /// Calculate total attack damage
    pub fn total_damage(&self) -> f32 {
        let weapon_total = self.base_damage + self.weapon_damage;
        weapon_total * (1.0 + self.mounted_bonus)
    }

    /// Calculate damage after armor mitigation
    pub fn mitigate_damage(&self, incoming_damage: f32) -> (f32, f32) {
        let mitigation = incoming_damage * self.armor_rating.min(0.95); // Max 95% reduction
        let final_damage = incoming_damage - mitigation;
        (final_damage, mitigation)
    }

    /// Roll for critical hit
    pub fn roll_critical(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen::<f32>() < self.crit_chance
    }

    /// Calculate final damage with critical hits
    pub fn calculate_final_damage(&self, is_critical: bool) -> f32 {
        let base = self.total_damage();
        if is_critical {
            base * self.crit_multiplier
        } else {
            base
        }
    }
}

/// Combat manager for handling combat in the world
#[derive(Debug, Clone)]
pub struct CombatManager {
    combat_log: Vec<CombatResult>,
    max_log_size: usize,
}

impl Default for CombatManager {
    fn default() -> Self {
        Self {
            combat_log: Vec::new(),
            max_log_size: 100,
        }
    }
}

impl CombatManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Execute combat between two entities
    pub fn execute_combat(
        &mut self,
        attacker_id: Uuid,
        defender_id: Uuid,
        attacker_stats: &CombatStats,
        defender_stats: &CombatStats,
        weapon_name: Option<String>,
    ) -> CombatResult {
        // Roll for critical hit
        let is_critical = attacker_stats.roll_critical();

        // Calculate base damage
        let raw_damage = attacker_stats.calculate_final_damage(is_critical);

        // Apply armor mitigation
        let (final_damage, mitigated) = defender_stats.mitigate_damage(raw_damage);

        // Create result
        let result = CombatResult {
            attacker_id,
            defender_id,
            damage_dealt: final_damage,
            damage_mitigated: mitigated,
            defender_killed: false, // Will be set by caller after applying damage
            weapon_used: weapon_name,
            critical_hit: is_critical,
        };

        // Add to combat log
        self.add_to_log(result.clone());

        result
    }

    /// Add combat result to log
    fn add_to_log(&mut self, result: CombatResult) {
        self.combat_log.push(result);

        // Keep log size manageable
        if self.combat_log.len() > self.max_log_size {
            self.combat_log.remove(0);
        }
    }

    /// Get recent combat log
    pub fn get_recent_combat(&self, count: usize) -> Vec<&CombatResult> {
        let start = self.combat_log.len().saturating_sub(count);
        self.combat_log[start..].iter().collect()
    }

    /// Clear combat log
    pub fn clear_log(&mut self) {
        self.combat_log.clear();
    }

    /// Get combat statistics
    pub fn get_combat_stats(&self, entity_id: &Uuid) -> CombatStatistics {
        let mut stats = CombatStatistics::default();

        for result in &self.combat_log {
            if result.attacker_id == *entity_id {
                stats.attacks_made += 1;
                stats.total_damage_dealt += result.damage_dealt;
                if result.defender_killed {
                    stats.kills += 1;
                }
                if result.critical_hit {
                    stats.critical_hits += 1;
                }
            }

            if result.defender_id == *entity_id {
                stats.times_attacked += 1;
                stats.total_damage_taken += result.damage_dealt;
                stats.total_damage_mitigated += result.damage_mitigated;
            }
        }

        stats
    }
}

/// Combat statistics for an entity
#[derive(Debug, Clone, Default)]
pub struct CombatStatistics {
    pub attacks_made: u32,
    pub times_attacked: u32,
    pub total_damage_dealt: f32,
    pub total_damage_taken: f32,
    pub total_damage_mitigated: f32,
    pub kills: u32,
    pub critical_hits: u32,
}

impl CombatStatistics {
    /// Calculate average damage per attack
    pub fn average_damage_dealt(&self) -> f32 {
        if self.attacks_made > 0 {
            self.total_damage_dealt / self.attacks_made as f32
        } else {
            0.0
        }
    }

    /// Calculate average damage taken per attack
    pub fn average_damage_taken(&self) -> f32 {
        if self.times_attacked > 0 {
            self.total_damage_taken / self.times_attacked as f32
        } else {
            0.0
        }
    }

    /// Calculate critical hit rate
    pub fn crit_rate(&self) -> f32 {
        if self.attacks_made > 0 {
            self.critical_hits as f32 / self.attacks_made as f32
        } else {
            0.0
        }
    }

    /// Calculate average mitigation per attack
    pub fn average_mitigation(&self) -> f32 {
        if self.times_attacked > 0 {
            self.total_damage_mitigated / self.times_attacked as f32
        } else {
            0.0
        }
    }

    /// Calculate kill/death ratio
    pub fn kd_ratio(&self) -> f32 {
        self.kills as f32
    }
}
