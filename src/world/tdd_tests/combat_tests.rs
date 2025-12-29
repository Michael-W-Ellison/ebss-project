// src/world/tdd_tests/combat_tests.rs
//! TDD tests for combat mechanics
//!
//! Tests combat system functionality including:
//! - Damage calculation
//! - Armor mitigation
//! - Critical hits
//! - Combat logging and statistics

use crate::world::combat::{CombatManager, CombatStats, CombatStatistics};
use uuid::Uuid;

#[test]
fn test_combat_stats_default() {
    let stats = CombatStats::default();

    assert_eq!(stats.base_damage, 5.0);
    assert_eq!(stats.weapon_damage, 0.0);
    assert_eq!(stats.armor_rating, 0.0);
    assert_eq!(stats.attack_speed, 1.0);
    assert_eq!(stats.crit_chance, 0.05);
    assert_eq!(stats.crit_multiplier, 1.5);
    assert_eq!(stats.mounted_bonus, 0.0);
}

#[test]
fn test_combat_stats_total_damage() {
    let mut stats = CombatStats::default();

    // Base damage only
    assert_eq!(stats.total_damage(), 5.0);

    // With weapon damage
    stats.weapon_damage = 10.0;
    assert_eq!(stats.total_damage(), 15.0);

    // With mounted bonus (25%)
    stats.mounted_bonus = 0.25;
    assert_eq!(stats.total_damage(), 18.75); // 15.0 * 1.25
}

#[test]
fn test_armor_mitigation() {
    let mut defender = CombatStats::default();

    // No armor
    let (final_damage, mitigated) = defender.mitigate_damage(10.0);
    assert_eq!(final_damage, 10.0);
    assert_eq!(mitigated, 0.0);

    // 50% armor
    defender.armor_rating = 0.5;
    let (final_damage, mitigated) = defender.mitigate_damage(10.0);
    assert_eq!(final_damage, 5.0);
    assert_eq!(mitigated, 5.0);

    // 100% armor (capped at 95%)
    defender.armor_rating = 1.0;
    let (final_damage, mitigated) = defender.mitigate_damage(10.0);
    assert_eq!(final_damage, 0.5);
    assert_eq!(mitigated, 9.5);
}

#[test]
fn test_critical_damage_multiplier() {
    let stats = CombatStats {
        base_damage: 10.0,
        crit_multiplier: 2.0,
        ..Default::default()
    };

    // Non-critical damage
    let normal = stats.calculate_final_damage(false);
    assert_eq!(normal, 10.0);

    // Critical damage
    let crit = stats.calculate_final_damage(true);
    assert_eq!(crit, 20.0);
}

#[test]
fn test_combat_manager_creation() {
    let manager = CombatManager::new();
    let recent = manager.get_recent_combat(10);
    assert!(recent.is_empty());
}

#[test]
fn test_combat_execution() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    let attacker_stats = CombatStats {
        base_damage: 10.0,
        weapon_damage: 5.0,
        crit_chance: 0.0, // No crit for predictable test
        ..Default::default()
    };

    let defender_stats = CombatStats {
        armor_rating: 0.2, // 20% armor
        ..Default::default()
    };

    let result = manager.execute_combat(
        attacker_id,
        defender_id,
        &attacker_stats,
        &defender_stats,
        Some("Iron Sword".to_string()),
    );

    assert_eq!(result.attacker_id, attacker_id);
    assert_eq!(result.defender_id, defender_id);
    // 15.0 total damage * 0.8 (after 20% armor) = 12.0
    assert_eq!(result.damage_dealt, 12.0);
    assert_eq!(result.damage_mitigated, 3.0);
    assert!(!result.critical_hit);
    assert_eq!(result.weapon_used, Some("Iron Sword".to_string()));
}

#[test]
fn test_combat_log() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    let attacker_stats = CombatStats::default();
    let defender_stats = CombatStats::default();

    // Execute multiple combats
    for _ in 0..5 {
        manager.execute_combat(
            attacker_id,
            defender_id,
            &attacker_stats,
            &defender_stats,
            None,
        );
    }

    let recent = manager.get_recent_combat(10);
    assert_eq!(recent.len(), 5);

    // Get only last 3
    let last_three = manager.get_recent_combat(3);
    assert_eq!(last_three.len(), 3);
}

#[test]
fn test_combat_log_clear() {
    let mut manager = CombatManager::new();

    let attacker_stats = CombatStats::default();
    let defender_stats = CombatStats::default();

    // Add some combat entries
    manager.execute_combat(
        Uuid::new_v4(),
        Uuid::new_v4(),
        &attacker_stats,
        &defender_stats,
        None,
    );

    assert!(!manager.get_recent_combat(10).is_empty());

    manager.clear_log();
    assert!(manager.get_recent_combat(10).is_empty());
}

#[test]
fn test_combat_statistics() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    let attacker_stats = CombatStats {
        base_damage: 10.0,
        crit_chance: 0.0, // No crits for predictable stats
        ..Default::default()
    };

    let defender_stats = CombatStats {
        armor_rating: 0.0,
        ..Default::default()
    };

    // Execute 3 combats
    for _ in 0..3 {
        manager.execute_combat(
            attacker_id,
            defender_id,
            &attacker_stats,
            &defender_stats,
            None,
        );
    }

    // Check attacker stats
    let attacker_combat_stats = manager.get_combat_stats(&attacker_id);
    assert_eq!(attacker_combat_stats.attacks_made, 3);
    assert_eq!(attacker_combat_stats.total_damage_dealt, 30.0); // 10 * 3
    assert_eq!(attacker_combat_stats.times_attacked, 0);

    // Check defender stats
    let defender_combat_stats = manager.get_combat_stats(&defender_id);
    assert_eq!(defender_combat_stats.times_attacked, 3);
    assert_eq!(defender_combat_stats.total_damage_taken, 30.0);
    assert_eq!(defender_combat_stats.attacks_made, 0);
}

#[test]
fn test_combat_statistics_averages() {
    let mut stats = CombatStatistics::default();

    // No attacks yet
    assert_eq!(stats.average_damage_dealt(), 0.0);
    assert_eq!(stats.average_damage_taken(), 0.0);
    assert_eq!(stats.crit_rate(), 0.0);
    assert_eq!(stats.average_mitigation(), 0.0);

    // Add some stats
    stats.attacks_made = 4;
    stats.total_damage_dealt = 100.0;
    stats.critical_hits = 1;

    stats.times_attacked = 2;
    stats.total_damage_taken = 30.0;
    stats.total_damage_mitigated = 10.0;

    assert_eq!(stats.average_damage_dealt(), 25.0);
    assert_eq!(stats.average_damage_taken(), 15.0);
    assert_eq!(stats.crit_rate(), 0.25);
    assert_eq!(stats.average_mitigation(), 5.0);
}

#[test]
fn test_combat_with_mounted_bonus() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    // Mounted attacker gets bonus
    let mounted_attacker = CombatStats {
        base_damage: 10.0,
        mounted_bonus: 0.5, // 50% bonus
        crit_chance: 0.0,
        ..Default::default()
    };

    let defender_stats = CombatStats::default();

    let result = manager.execute_combat(
        attacker_id,
        defender_id,
        &mounted_attacker,
        &defender_stats,
        None,
    );

    // 10.0 * 1.5 (mounted) = 15.0
    assert_eq!(result.damage_dealt, 15.0);
}

#[test]
fn test_combat_high_armor_mitigation() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    let attacker_stats = CombatStats {
        base_damage: 100.0,
        crit_chance: 0.0,
        ..Default::default()
    };

    // Very high armor (95% cap applies)
    let heavily_armored = CombatStats {
        armor_rating: 0.99,
        ..Default::default()
    };

    let result = manager.execute_combat(
        attacker_id,
        defender_id,
        &attacker_stats,
        &heavily_armored,
        None,
    );

    // Should be capped at 95% reduction, not 99%
    assert_eq!(result.damage_dealt, 5.0);
    assert_eq!(result.damage_mitigated, 95.0);
}

#[test]
fn test_combat_kills_tracking() {
    let stats = CombatStatistics {
        kills: 5,
        ..Default::default()
    };

    assert_eq!(stats.kd_ratio(), 5.0);
}

#[test]
fn test_combat_manager_log_size_limit() {
    let mut manager = CombatManager::new();

    let attacker_stats = CombatStats::default();
    let defender_stats = CombatStats::default();

    // Add more than max_log_size (100) entries
    for _ in 0..150 {
        manager.execute_combat(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &attacker_stats,
            &defender_stats,
            None,
        );
    }

    // Should be limited to 100
    let all = manager.get_recent_combat(200);
    assert_eq!(all.len(), 100);
}
