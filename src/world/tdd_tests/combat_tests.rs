// src/world/tdd_tests/combat_tests.rs
//! TDD tests for combat mechanics
//!
//! Tests combat system integration including:
//! - CombatStats calculations (damage, armor, crits)
//! - CombatManager operations (execute, log, statistics)
//! - Agent vs Animal combat
//! - Animal vs Agent combat
//! - Agent vs Agent combat
//! - Animal vs Animal combat

use crate::world::combat::{CombatAction, CombatManager, CombatResult, CombatStats, CombatStatistics};
use crate::world::{World, WorldConfig};
use uuid::Uuid;

// ===== CombatStats Tests =====

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

    // Add weapon damage
    stats.weapon_damage = 10.0;
    assert_eq!(stats.total_damage(), 15.0);

    // Add mounted bonus (50%)
    stats.mounted_bonus = 0.5;
    assert_eq!(stats.total_damage(), 22.5); // 15 * 1.5
}

#[test]
fn test_combat_stats_mitigate_damage_no_armor() {
    let stats = CombatStats::default();

    let (final_damage, mitigated) = stats.mitigate_damage(20.0);

    assert_eq!(final_damage, 20.0);
    assert_eq!(mitigated, 0.0);
}

#[test]
fn test_combat_stats_mitigate_damage_with_armor() {
    let mut stats = CombatStats::default();
    stats.armor_rating = 0.5; // 50% damage reduction

    let (final_damage, mitigated) = stats.mitigate_damage(20.0);

    assert_eq!(mitigated, 10.0);
    assert_eq!(final_damage, 10.0);
}

#[test]
fn test_combat_stats_armor_cap_at_95_percent() {
    let mut stats = CombatStats::default();
    stats.armor_rating = 1.0; // 100% armor requested

    let (final_damage, mitigated) = stats.mitigate_damage(100.0);

    // Max mitigation is 95%
    assert_eq!(mitigated, 95.0);
    assert_eq!(final_damage, 5.0);
}

#[test]
fn test_combat_stats_calculate_final_damage_no_crit() {
    let mut stats = CombatStats::default();
    stats.base_damage = 10.0;
    stats.weapon_damage = 5.0;

    let damage = stats.calculate_final_damage(false);
    assert_eq!(damage, 15.0);
}

#[test]
fn test_combat_stats_calculate_final_damage_with_crit() {
    let mut stats = CombatStats::default();
    stats.base_damage = 10.0;
    stats.weapon_damage = 5.0;
    stats.crit_multiplier = 2.0;

    let damage = stats.calculate_final_damage(true);
    assert_eq!(damage, 30.0); // 15 * 2.0
}

#[test]
fn test_combat_action_variants_exist() {
    // Ensure all combat action variants are accessible
    let _melee = CombatAction::MeleeAttack;
    let _ranged = CombatAction::RangedAttack;
    let _heavy = CombatAction::HeavyAttack;
    let _quick = CombatAction::QuickAttack;
    let _defend = CombatAction::Defend;

    assert_eq!(CombatAction::MeleeAttack, CombatAction::MeleeAttack);
    assert_ne!(CombatAction::MeleeAttack, CombatAction::RangedAttack);
}

// ===== CombatManager Tests =====

#[test]
fn test_combat_manager_creation() {
    let manager = CombatManager::new();

    // Should start with empty log
    assert!(manager.get_recent_combat(10).is_empty());
}

#[test]
fn test_combat_manager_execute_combat_basic() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    let attacker_stats = CombatStats {
        base_damage: 10.0,
        weapon_damage: 5.0,
        crit_chance: 0.0, // No crits for predictable test
        ..Default::default()
    };

    let defender_stats = CombatStats::default();

    let result = manager.execute_combat(
        attacker_id,
        defender_id,
        &attacker_stats,
        &defender_stats,
        Some("Test Sword".to_string()),
    );

    assert_eq!(result.attacker_id, attacker_id);
    assert_eq!(result.defender_id, defender_id);
    assert_eq!(result.damage_dealt, 15.0); // 10 + 5 base damage
    assert_eq!(result.damage_mitigated, 0.0); // No armor
    assert!(!result.defender_killed); // Not set by execute_combat
    assert_eq!(result.weapon_used, Some("Test Sword".to_string()));
    assert!(!result.critical_hit); // 0% crit chance
}

#[test]
fn test_combat_manager_execute_combat_with_armor() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    let attacker_stats = CombatStats {
        base_damage: 20.0,
        crit_chance: 0.0,
        ..Default::default()
    };

    let defender_stats = CombatStats {
        armor_rating: 0.5, // 50% damage reduction
        ..Default::default()
    };

    let result = manager.execute_combat(
        attacker_id,
        defender_id,
        &attacker_stats,
        &defender_stats,
        None,
    );

    assert_eq!(result.damage_dealt, 10.0); // 20 * 0.5 after armor
    assert_eq!(result.damage_mitigated, 10.0);
}

#[test]
fn test_combat_manager_logs_combat() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();
    let attacker_stats = CombatStats::default();
    let defender_stats = CombatStats::default();

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

    let log = manager.get_recent_combat(10);
    assert_eq!(log.len(), 3);
}

#[test]
fn test_combat_manager_log_size_limit() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();
    let attacker_stats = CombatStats::default();
    let defender_stats = CombatStats::default();

    // Execute 150 combats (exceeds default max of 100)
    for _ in 0..150 {
        manager.execute_combat(
            attacker_id,
            defender_id,
            &attacker_stats,
            &defender_stats,
            None,
        );
    }

    let log = manager.get_recent_combat(200);
    assert_eq!(log.len(), 100); // Capped at max log size
}

#[test]
fn test_combat_manager_clear_log() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();
    let attacker_stats = CombatStats::default();
    let defender_stats = CombatStats::default();

    manager.execute_combat(attacker_id, defender_id, &attacker_stats, &defender_stats, None);
    assert_eq!(manager.get_recent_combat(10).len(), 1);

    manager.clear_log();
    assert!(manager.get_recent_combat(10).is_empty());
}

#[test]
fn test_combat_manager_get_combat_stats_as_attacker() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    let attacker_stats = CombatStats {
        base_damage: 10.0,
        crit_chance: 0.0,
        ..Default::default()
    };
    let defender_stats = CombatStats::default();

    // Execute 5 attacks
    for _ in 0..5 {
        manager.execute_combat(attacker_id, defender_id, &attacker_stats, &defender_stats, None);
    }

    let stats = manager.get_combat_stats(&attacker_id);

    assert_eq!(stats.attacks_made, 5);
    assert_eq!(stats.times_attacked, 0);
    assert_eq!(stats.total_damage_dealt, 50.0); // 5 * 10
    assert_eq!(stats.total_damage_taken, 0.0);
}

#[test]
fn test_combat_manager_get_combat_stats_as_defender() {
    let mut manager = CombatManager::new();

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    let attacker_stats = CombatStats {
        base_damage: 20.0,
        crit_chance: 0.0,
        ..Default::default()
    };
    let defender_stats = CombatStats {
        armor_rating: 0.25,
        ..Default::default()
    };

    // Execute 4 attacks
    for _ in 0..4 {
        manager.execute_combat(attacker_id, defender_id, &attacker_stats, &defender_stats, None);
    }

    let stats = manager.get_combat_stats(&defender_id);

    assert_eq!(stats.attacks_made, 0);
    assert_eq!(stats.times_attacked, 4);
    assert_eq!(stats.total_damage_taken, 60.0); // 4 * (20 * 0.75)
    assert_eq!(stats.total_damage_mitigated, 20.0); // 4 * 5
}

// ===== CombatStatistics Tests =====

#[test]
fn test_combat_statistics_average_damage_dealt() {
    let stats = CombatStatistics {
        attacks_made: 5,
        total_damage_dealt: 100.0,
        ..Default::default()
    };

    assert_eq!(stats.average_damage_dealt(), 20.0);
}

#[test]
fn test_combat_statistics_average_damage_dealt_no_attacks() {
    let stats = CombatStatistics::default();

    assert_eq!(stats.average_damage_dealt(), 0.0);
}

#[test]
fn test_combat_statistics_average_damage_taken() {
    let stats = CombatStatistics {
        times_attacked: 10,
        total_damage_taken: 80.0,
        ..Default::default()
    };

    assert_eq!(stats.average_damage_taken(), 8.0);
}

#[test]
fn test_combat_statistics_crit_rate() {
    let stats = CombatStatistics {
        attacks_made: 20,
        critical_hits: 5,
        ..Default::default()
    };

    assert_eq!(stats.crit_rate(), 0.25);
}

#[test]
fn test_combat_statistics_average_mitigation() {
    let stats = CombatStatistics {
        times_attacked: 4,
        total_damage_mitigated: 40.0,
        ..Default::default()
    };

    assert_eq!(stats.average_mitigation(), 10.0);
}

#[test]
fn test_combat_statistics_kd_ratio() {
    let stats = CombatStatistics {
        kills: 3,
        ..Default::default()
    };

    assert_eq!(stats.kd_ratio(), 3.0);
}

// ===== World Combat Integration Tests =====

#[test]
fn test_world_agent_attack_animal() {
    let mut world = World::new(WorldConfig::default());

    // Spawn a wolf (predator with good stats)
    let animal_id = world.spawn_animal("wolf".to_string(), (25, 25)).unwrap();

    // Get initial health
    let initial_health = world.animals.get(&animal_id).unwrap().current_health;

    // Create a test agent ID
    let agent_id = Uuid::new_v4();

    // Execute combat
    let result = world.agent_attack_animal(
        agent_id,
        10.0, // weapon damage
        0.0,  // no mounted bonus
        &animal_id,
    ).unwrap();

    // Verify damage was dealt
    assert!(result.damage_dealt > 0.0);

    // Verify animal took damage
    let current_health = world.animals.get(&animal_id).unwrap().current_health;
    assert!(current_health < initial_health);

    // Damage applied should match result
    let damage_taken = initial_health - current_health;
    assert!((damage_taken - result.damage_dealt).abs() < 0.01);
}

#[test]
fn test_world_agent_attack_animal_with_mounted_bonus() {
    let mut world = World::new(WorldConfig::default());

    let animal_id = world.spawn_animal("deer".to_string(), (25, 25)).unwrap();
    let agent_id = Uuid::new_v4();

    // Attack without mounted bonus
    let result_unmounted = world.agent_attack_animal(
        agent_id,
        10.0, // weapon damage
        0.0,  // no mounted bonus
        &animal_id,
    ).unwrap();

    // Spawn another animal
    let animal_id2 = world.spawn_animal("deer".to_string(), (30, 30)).unwrap();

    // Attack with mounted bonus
    let result_mounted = world.agent_attack_animal(
        agent_id,
        10.0, // weapon damage
        0.5,  // 50% mounted bonus
        &animal_id2,
    ).unwrap();

    // Mounted attack should deal more damage
    assert!(result_mounted.damage_dealt > result_unmounted.damage_dealt);
}

#[test]
fn test_world_agent_attack_animal_kills_animal() {
    let mut world = World::new(WorldConfig::default());

    // Spawn a rabbit (low health animal)
    let animal_id = world.spawn_animal("rabbit".to_string(), (25, 25)).unwrap();
    let agent_id = Uuid::new_v4();

    // Attack with high damage to kill
    let result = world.agent_attack_animal(
        agent_id,
        100.0, // massive weapon damage
        0.0,
        &animal_id,
    ).unwrap();

    // Should be marked as killed
    assert!(result.defender_killed);

    // Animal should be dead
    let animal = world.animals.get(&animal_id).unwrap();
    assert!(!animal.is_alive());
}

#[test]
fn test_world_agent_attack_animal_not_found() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = Uuid::new_v4();
    let fake_animal_id = Uuid::new_v4();

    let result = world.agent_attack_animal(agent_id, 10.0, 0.0, &fake_animal_id);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Animal not found");
}

#[test]
fn test_world_animal_attack_agent() {
    let mut world = World::new(WorldConfig::default());

    // Spawn a bear (large predator - more damage based on max_health)
    let animal_id = world.spawn_animal("bear".to_string(), (25, 25)).unwrap();
    let agent_id = Uuid::new_v4();

    // Animal attacks agent with no armor
    let result = world.animal_attack_agent(
        &animal_id,
        agent_id,
        0.0, // no armor
    ).unwrap();

    // Animal damage is based on max_health/20, capped at 20
    assert!(result.damage_dealt > 0.0);
    assert_eq!(result.damage_mitigated, 0.0);
    assert!(result.weapon_used.as_ref().unwrap().contains("attack"));
}

#[test]
fn test_world_animal_attack_agent_with_armor() {
    let mut world = World::new(WorldConfig::default());

    let animal_id = world.spawn_animal("wolf".to_string(), (25, 25)).unwrap();
    let agent_id = Uuid::new_v4();

    // Agent has 50% armor
    let result = world.animal_attack_agent(
        &animal_id,
        agent_id,
        0.5, // 50% armor
    ).unwrap();

    // Damage should be reduced
    assert!(result.damage_mitigated > 0.0);
    // Original damage should be higher than dealt damage
    let original_damage = result.damage_dealt + result.damage_mitigated;
    assert!(original_damage > result.damage_dealt);
}

#[test]
fn test_world_agent_attack_agent() {
    let mut world = World::new(WorldConfig::default());

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    let result = world.agent_attack_agent(
        attacker_id,
        defender_id,
        15.0, // attacker weapon
        0.2,  // attacker armor
        0.0,  // attacker mounted
        5.0,  // defender weapon
        0.3,  // defender armor
        0.0,  // defender mounted
    ).unwrap();

    // Attacker deals damage to defender
    assert!(result.damage_dealt > 0.0);
    assert_eq!(result.attacker_id, attacker_id);
    assert_eq!(result.defender_id, defender_id);

    // Defender's armor should mitigate some damage
    // Attacker total damage: 5 (base) + 15 (weapon) = 20
    // Defender armor: 30% mitigation
    // Expected dealt: 20 * 0.7 = 14, mitigated: 6
    assert!(result.damage_mitigated > 0.0);
}

#[test]
fn test_world_agent_attack_agent_with_mounted_bonus() {
    let mut world = World::new(WorldConfig::default());

    let attacker_id = Uuid::new_v4();
    let defender_id = Uuid::new_v4();

    // Run multiple trials to average out critical hit randomness
    let mut unmounted_total = 0.0;
    let mut mounted_total = 0.0;

    for _ in 0..20 {
        // Unmounted attack
        let result_unmounted = world.agent_attack_agent(
            attacker_id,
            defender_id,
            10.0, 0.0, 0.0, // attacker
            0.0, 0.0, 0.0,  // defender
        ).unwrap();
        unmounted_total += result_unmounted.damage_dealt;

        // Mounted attack
        let result_mounted = world.agent_attack_agent(
            attacker_id,
            defender_id,
            10.0, 0.0, 0.5, // 50% mounted bonus
            0.0, 0.0, 0.0,
        ).unwrap();
        mounted_total += result_mounted.damage_dealt;
    }

    // On average, mounted should deal more damage
    assert!(mounted_total > unmounted_total);
}

#[test]
fn test_world_animal_attack_animal() {
    let mut world = World::new(WorldConfig::default());

    let attacker_id = world.spawn_animal("wolf".to_string(), (25, 25)).unwrap();
    let defender_id = world.spawn_animal("deer".to_string(), (26, 25)).unwrap();

    let initial_health = world.animals.get(&defender_id).unwrap().current_health;

    let result = world.animal_attack_animal(&attacker_id, &defender_id).unwrap();

    assert!(result.damage_dealt > 0.0);

    // Defender should have taken damage
    let current_health = world.animals.get(&defender_id).unwrap().current_health;
    assert!(current_health < initial_health);
}

#[test]
fn test_world_animal_attack_animal_kills_prey() {
    let mut world = World::new(WorldConfig::default());

    // Strong predator (bear)
    let attacker_id = world.spawn_animal("bear".to_string(), (25, 25)).unwrap();

    // Weak prey (rabbit)
    let defender_id = world.spawn_animal("rabbit".to_string(), (26, 25)).unwrap();

    // Attack until dead
    let mut killed = false;
    for _ in 0..10 {
        let result = world.animal_attack_animal(&attacker_id, &defender_id).unwrap();
        if result.defender_killed {
            killed = true;
            break;
        }
    }

    assert!(killed);
    assert!(!world.animals.get(&defender_id).unwrap().is_alive());
}

#[test]
fn test_world_animal_attack_animal_not_found() {
    let mut world = World::new(WorldConfig::default());

    let real_id = world.spawn_animal("wolf".to_string(), (25, 25)).unwrap();
    let fake_id = Uuid::new_v4();

    // Attacker not found
    let result = world.animal_attack_animal(&fake_id, &real_id);
    assert!(result.is_err());

    // Defender not found
    let result = world.animal_attack_animal(&real_id, &fake_id);
    assert!(result.is_err());
}

#[test]
fn test_world_get_combat_stats() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = Uuid::new_v4();

    // Spawn some animals to attack
    for i in 0..3 {
        let animal_id = world.spawn_animal("deer".to_string(), (25 + i, 25)).unwrap();
        world.agent_attack_animal(agent_id, 10.0, 0.0, &animal_id).unwrap();
    }

    let stats = world.get_combat_stats(&agent_id);

    assert_eq!(stats.attacks_made, 3);
    assert!(stats.total_damage_dealt > 0.0);
}

#[test]
fn test_world_get_recent_combat() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = Uuid::new_v4();

    // Perform some combat
    for i in 0..5 {
        let animal_id = world.spawn_animal("deer".to_string(), (25 + i, 25)).unwrap();
        world.agent_attack_animal(agent_id, 10.0, 0.0, &animal_id).unwrap();
    }

    let recent = world.get_recent_combat(3);
    assert_eq!(recent.len(), 3);

    let all = world.get_recent_combat(10);
    assert_eq!(all.len(), 5);
}

#[test]
fn test_world_damage_animal_directly() {
    let mut world = World::new(WorldConfig::default());

    let animal_id = world.spawn_animal("deer".to_string(), (25, 25)).unwrap();
    let initial_health = world.animals.get(&animal_id).unwrap().current_health;

    // Damage but not kill
    let is_dead = world.damage_animal(&animal_id, 10.0).unwrap();
    assert!(!is_dead);

    let health = world.animals.get(&animal_id).unwrap().current_health;
    assert_eq!(health, initial_health - 10.0);

    // Kill with massive damage
    let is_dead = world.damage_animal(&animal_id, 1000.0).unwrap();
    assert!(is_dead);
}

// ===== Combat Result Tests =====

#[test]
fn test_combat_result_structure() {
    let result = CombatResult {
        attacker_id: Uuid::new_v4(),
        defender_id: Uuid::new_v4(),
        damage_dealt: 15.0,
        damage_mitigated: 5.0,
        defender_killed: false,
        weapon_used: Some("Iron Sword".to_string()),
        critical_hit: true,
    };

    assert_eq!(result.damage_dealt, 15.0);
    assert_eq!(result.damage_mitigated, 5.0);
    assert!(!result.defender_killed);
    assert!(result.critical_hit);
    assert_eq!(result.weapon_used, Some("Iron Sword".to_string()));
}

#[test]
fn test_combat_result_serialization() {
    let result = CombatResult {
        attacker_id: Uuid::new_v4(),
        defender_id: Uuid::new_v4(),
        damage_dealt: 25.0,
        damage_mitigated: 10.0,
        defender_killed: true,
        weapon_used: None,
        critical_hit: false,
    };

    // Should be serializable
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(serialized.contains("25.0"));

    // Should be deserializable
    let deserialized: CombatResult = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.damage_dealt, 25.0);
    assert!(deserialized.defender_killed);
}

// ===== Edge Case Tests =====

#[test]
fn test_combat_zero_damage() {
    let mut manager = CombatManager::new();

    let attacker_stats = CombatStats {
        base_damage: 0.0,
        weapon_damage: 0.0,
        crit_chance: 0.0,
        ..Default::default()
    };

    let defender_stats = CombatStats::default();

    let result = manager.execute_combat(
        Uuid::new_v4(),
        Uuid::new_v4(),
        &attacker_stats,
        &defender_stats,
        None,
    );

    assert_eq!(result.damage_dealt, 0.0);
}

#[test]
fn test_combat_extreme_armor() {
    let mut manager = CombatManager::new();

    let attacker_stats = CombatStats {
        base_damage: 100.0,
        crit_chance: 0.0,
        ..Default::default()
    };

    let defender_stats = CombatStats {
        armor_rating: 0.95, // Max armor
        ..Default::default()
    };

    let result = manager.execute_combat(
        Uuid::new_v4(),
        Uuid::new_v4(),
        &attacker_stats,
        &defender_stats,
        None,
    );

    assert_eq!(result.damage_dealt, 5.0); // Only 5% gets through
    assert_eq!(result.damage_mitigated, 95.0);
}

#[test]
fn test_combat_multiple_participants_tracked() {
    let mut manager = CombatManager::new();

    let entity_a = Uuid::new_v4();
    let entity_b = Uuid::new_v4();
    let entity_c = Uuid::new_v4();

    let stats = CombatStats {
        base_damage: 10.0,
        crit_chance: 0.0,
        ..Default::default()
    };

    // A attacks B
    manager.execute_combat(entity_a, entity_b, &stats, &stats, None);
    // A attacks C
    manager.execute_combat(entity_a, entity_c, &stats, &stats, None);
    // B attacks A
    manager.execute_combat(entity_b, entity_a, &stats, &stats, None);

    let stats_a = manager.get_combat_stats(&entity_a);
    assert_eq!(stats_a.attacks_made, 2);
    assert_eq!(stats_a.times_attacked, 1);

    let stats_b = manager.get_combat_stats(&entity_b);
    assert_eq!(stats_b.attacks_made, 1);
    assert_eq!(stats_b.times_attacked, 1);

    let stats_c = manager.get_combat_stats(&entity_c);
    assert_eq!(stats_c.attacks_made, 0);
    assert_eq!(stats_c.times_attacked, 1);
}
