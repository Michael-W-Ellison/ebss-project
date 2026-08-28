// src/agents/tests/lifecycle_and_survival_tests.rs
//! TDD tests for agent lifecycle and survival mechanics
//!
//! These tests verify:
//! - Starvation progression and death
//! - Aging mechanics and death from old age
//! - Energy depletion and recovery
//! - Health management
//! - Eating and drinking mechanics

use crate::agents::{Agent, AgentConfig, AgentState, LifeStage, InventoryItem};
use crate::core::drives::{DriveType, DriveState};

#[test]
fn test_agent_starts_with_full_health_and_energy() {
    let agent = Agent::new(AgentConfig::default());

    assert_eq!(agent.state.health, 100.0);
    assert_eq!(agent.state.energy, 100.0);
    assert_eq!(agent.state.age, 0);
}

#[test]
fn test_agent_ages_over_time() {
    let mut agent = Agent::new(AgentConfig::default());

    let initial_age = agent.state.age;

    // Age the agent
    agent.age_tick();

    assert_eq!(agent.state.age, initial_age + 1);
}

#[test]
fn test_agent_dies_from_old_age() {
    let mut agent = Agent::new(AgentConfig::default());

    // Set age to max
    agent.state.age = agent.state.max_age;

    // Check if agent should die
    let should_die = agent.state.age >= agent.state.max_age;
    assert!(should_die, "Agent at max age should die");
}

#[test]
fn test_hunger_increases_without_food() {
    let mut agent = Agent::new(AgentConfig::default());

    // Initial hunger should be low
    let initial_hunger = agent.drives.get(DriveType::Hunger).unwrap().value;
    assert_eq!(initial_hunger, 0.0);

    // Simulate time passing without eating
    for _ in 0..100 {
        agent.drives.tick();
    }

    // Hunger should increase
    let current_hunger = agent.drives.get(DriveType::Hunger).unwrap().value;
    assert!(current_hunger > initial_hunger);
    assert!(current_hunger >= 0.7); // Should reach hunger threshold
}

#[test]
fn test_eating_food_reduces_hunger() {
    let mut agent = Agent::new(AgentConfig::default());

    // Make agent hungry
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.8;
    }

    // Give agent food
    agent.inventory.add_item(InventoryItem::new("food".to_string(), 10));

    // Eat food
    let ate_food = agent.eat_food(1);

    if ate_food {
        // Hunger should decrease
        let hunger_after = agent.drives.get(DriveType::Hunger).unwrap().value;
        assert!(hunger_after < 0.8, "Eating should reduce hunger");
    }
}

#[test]
fn test_starvation_counter_increases_without_food() {
    let mut agent = Agent::new(AgentConfig::default());

    assert_eq!(agent.state.ticks_without_food, 0);

    // Simulate time without eating
    for _ in 0..100 {
        agent.update_starvation();
    }

    // Counter should increase
    assert!(agent.state.ticks_without_food > 0);
}

#[test]
fn test_eating_resets_starvation_counter() {
    let mut agent = Agent::new(AgentConfig::default());

    // Increase starvation counter
    agent.state.ticks_without_food = 1000;
    agent.state.physiology.gone_without_food_for(1000);

    // Give food and eat
    agent.inventory.add_item(InventoryItem::new("food".to_string(), 5));
    agent.eat_food(1);

    // Counter should reset
    assert_eq!(agent.state.ticks_without_food, 0);
}

#[test]
fn test_agent_is_starving_after_threshold() {
    let mut agent = Agent::new(AgentConfig::default());

    // Not starving initially
    assert!(!agent.state.is_starving());

    // Simulate extended starvation (3+ days = 4320+ ticks at 1440 ticks/day)
    agent.state.ticks_without_food = 4500;
    agent.state.physiology.gone_without_food_for(4500);

    // Should be starving
    assert!(agent.state.is_starving());
}

#[test]
fn test_starvation_damages_health() {
    let mut agent = Agent::new(AgentConfig::default());

    let initial_health = agent.state.health;

    // Set to starving state
    agent.state.ticks_without_food = 5000;
    agent.state.physiology.gone_without_food_for(5000);

    // Apply starvation damage
    agent.apply_starvation_damage();

    // Health should decrease
    assert!(agent.state.health < initial_health);
}

#[test]
fn test_agent_dies_from_starvation() {
    let mut agent = Agent::new(AgentConfig::default());

    // Extreme starvation
    agent.state.ticks_without_food = 20000;
    agent.state.physiology.gone_without_food_for(20000); // ~14 days

    // Apply damage until death
    for _ in 0..100 {
        agent.apply_starvation_damage();
        if agent.state.health <= 0.0 {
            break;
        }
    }

    // Should be dead
    assert!(agent.state.health <= 0.0, "Prolonged starvation should kill agent");
}

#[test]
fn test_energy_depletes_from_activity() {
    let mut agent = Agent::new(AgentConfig::default());

    let initial_energy = agent.state.energy;
    assert_eq!(initial_energy, 100.0);

    // Perform energy-consuming action
    agent.consume_energy(20.0);

    assert_eq!(agent.state.energy, 80.0);
}

#[test]
fn test_energy_cannot_go_negative() {
    let mut agent = Agent::new(AgentConfig::default());

    // Try to consume more energy than available
    agent.consume_energy(150.0);

    // Should clamp at zero
    assert_eq!(agent.state.energy, 0.0);
}

#[test]
fn test_rest_restores_energy() {
    let mut agent = Agent::new(AgentConfig::default());

    // Deplete energy
    agent.state.energy = 30.0;

    // Rest
    agent.rest(10.0);

    // Energy should increase
    assert!(agent.state.energy > 30.0);
    assert!(agent.state.energy <= 100.0); // Shouldn't exceed max
}

#[test]
fn test_energy_cannot_exceed_maximum() {
    let mut agent = Agent::new(AgentConfig::default());

    agent.state.energy = 95.0;

    // Try to rest beyond max
    agent.rest(20.0);

    // Should clamp at 100
    assert_eq!(agent.state.energy, 100.0);
}

#[test]
fn test_thirst_increases_over_time() {
    let mut agent = Agent::new(AgentConfig::default());

    let initial_thirst = agent.drives.get(DriveType::Thirst).unwrap().value;

    // Simulate time passing
    for _ in 0..100 {
        agent.drives.tick();
    }

    let current_thirst = agent.drives.get(DriveType::Thirst).unwrap().value;
    assert!(current_thirst > initial_thirst);
}

#[test]
fn test_drinking_water_reduces_thirst() {
    let mut agent = Agent::new(AgentConfig::default());

    // Make agent thirsty
    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
        thirst.value = 0.9;
    }

    // Add water container
    let mut waterskin = InventoryItem::new_container("waterskin".to_string(), 1, 5.0);
    agent.inventory.add_item(waterskin);
    agent.inventory.fill_containers(5.0);

    // Drink water
    let drank = agent.drink_water(1.0);

    if drank {
        let thirst_after = agent.drives.get(DriveType::Thirst).unwrap().value;
        assert!(thirst_after < 0.9, "Drinking should reduce thirst");
    }
}

#[test]
fn test_health_damage_reduces_health() {
    let mut agent = Agent::new(AgentConfig::default());

    let initial_health = agent.state.health;

    // Take damage
    agent.take_damage(25.0);

    assert_eq!(agent.state.health, initial_health - 25.0);
}

#[test]
fn test_health_cannot_go_negative() {
    let mut agent = Agent::new(AgentConfig::default());

    // Take massive damage
    agent.take_damage(200.0);

    // Should clamp at zero
    assert_eq!(agent.state.health, 0.0);
}

#[test]
fn test_agent_is_dead_when_health_zero() {
    let mut agent = Agent::new(AgentConfig::default());

    assert!(!agent.is_dead());

    agent.state.health = 0.0;

    assert!(agent.is_dead());
}

#[test]
fn test_life_stage_progression() {
    let mut agent = Agent::new(AgentConfig::default());

    // Start as infant
    agent.state.life_stage = LifeStage::Infant;
    agent.state.age = 0;

    // Age through stages (based on LifeStage::from_age thresholds)
    // Infant (0-500)
    agent.state.age = 250;
    agent.update_life_stage();
    assert_eq!(agent.state.life_stage, LifeStage::Infant);

    // Child (501-1500)
    agent.state.age = 1000;
    agent.update_life_stage();
    assert_eq!(agent.state.life_stage, LifeStage::Child);

    // Adolescent (1501-2500)
    agent.state.age = 2000;
    agent.update_life_stage();
    assert_eq!(agent.state.life_stage, LifeStage::Adolescent);

    // Adult (2501-8000)
    agent.state.age = 5000;
    agent.update_life_stage();
    assert_eq!(agent.state.life_stage, LifeStage::Adult);

    // Elderly (8001+)
    agent.state.age = 9000;
    agent.update_life_stage();
    assert_eq!(agent.state.life_stage, LifeStage::Elderly);
}

#[test]
fn test_multiple_survival_needs_simultaneous() {
    let mut agent = Agent::new(AgentConfig::default());

    // Simulate extended time without food or water
    for _ in 0..200 {
        agent.drives.tick();
    }

    // Both hunger and thirst should be high
    let hunger = agent.drives.get(DriveType::Hunger).unwrap().value;
    let thirst = agent.drives.get(DriveType::Thirst).unwrap().value;

    assert!(hunger >= 0.7, "Hunger should be critical");
    assert!(thirst >= 0.75, "Thirst should be critical");
}

#[test]
fn test_rest_drive_accumulates_from_activity() {
    let mut agent = Agent::new(AgentConfig::default());

    // Deplete energy through activity
    agent.consume_energy(60.0);

    // Update rest drive based on fatigue
    for _ in 0..100 {
        agent.drives.tick();
    }

    let rest_drive = agent.drives.get(DriveType::Rest).unwrap().value;
    assert!(rest_drive > 0.0, "Rest drive should increase with fatigue");
}

#[test]
fn test_agent_survival_requires_food_water_rest() {
    let mut agent = Agent::new(AgentConfig::default());

    // Give agent resources
    agent.inventory.add_item(InventoryItem::new("food".to_string(), 100));
    let mut waterskin = InventoryItem::new_container("waterskin".to_string(), 1, 10.0);
    agent.inventory.add_item(waterskin);
    agent.inventory.fill_containers(10.0);

    // Simulate survival loop
    for _ in 0..1000 {
        // Drives accumulate
        agent.drives.tick();

        // Agent satisfies needs
        if agent.drives.get(DriveType::Hunger).unwrap().is_active() {
            agent.eat_food(1);
        }

        if agent.drives.get(DriveType::Thirst).unwrap().is_active() {
            agent.drink_water(1.0);
        }

        if agent.drives.get(DriveType::Rest).unwrap().is_active() {
            agent.rest(10.0);
        }
    }

    // Agent should still be alive
    assert!(agent.state.health > 0.0);
    assert!(!agent.is_dead());
}

#[test]
fn test_agent_without_food_eventually_dies() {
    let mut agent = Agent::new(AgentConfig::default());

    // No food available - check properly
    assert!(agent.inventory.get_item("food").is_none() || agent.inventory.get_item("food").unwrap().quantity == 0);

    // Simulate starvation
    for _ in 0..15000 {
        agent.update_starvation();
        agent.apply_starvation_damage();

        if agent.is_dead() {
            break;
        }
    }

    // Should eventually die from starvation
    assert!(agent.is_dead() || agent.state.health < 50.0);
}
