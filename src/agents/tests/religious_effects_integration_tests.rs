// src/agents/tests/religious_effects_integration_tests.rs
//! Integration tests for religious building happiness effects

use crate::agents::{Agent, AgentConfig, Trait};
use crate::agents::religious_effects::{
    calculate_religious_effects, total_happiness_modifier,
    should_seek_religious_building, should_avoid_religious_building,
    RELIGIOUS_EFFECT_RADIUS,
};
use crate::world::{BuildingType, Position};

#[test]
fn test_believer_agent_happiness_near_shrine() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Believer);
    agent.state.position = (5, 5, 0);

    let initial_happiness = agent.emotions.happiness;

    // Create a shrine at the agent's location
    let buildings = vec![
        (Position::new(5, 5), BuildingType::Shrine, true),
    ];

    let effects = calculate_religious_effects(
        Position::new(5, 5),
        &agent.traits,
        &buildings,
        0, // no other believers
    );

    let modifier = total_happiness_modifier(&effects);

    // Apply the effect
    agent.apply_religious_happiness(modifier, "Test shrine effect");

    // Believer should have increased happiness
    assert!(agent.emotions.happiness > initial_happiness);
}

#[test]
fn test_atheist_agent_discomfort_near_temple() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Atheist);
    agent.state.position = (5, 5, 0);
    agent.emotions.happiness = 0.8; // Start with some happiness

    // Create a temple at the agent's location
    let buildings = vec![
        (Position::new(5, 5), BuildingType::Temple, true),
    ];

    let effects = calculate_religious_effects(
        Position::new(5, 5),
        &agent.traits,
        &buildings,
        0,
    );

    let modifier = total_happiness_modifier(&effects);

    // Modifier should be negative
    assert!(modifier < 0.0);

    // Apply the effect
    agent.apply_religious_happiness(modifier, "Test temple discomfort");

    // Atheist should have decreased happiness
    assert!(agent.emotions.happiness < 0.8);
}

#[test]
fn test_zealot_gets_community_bonus() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Zealot);

    // Temple at agent's position
    let buildings = vec![
        (Position::new(0, 0), BuildingType::Temple, true),
    ];

    // Without other believers
    let effects_alone = calculate_religious_effects(
        Position::new(0, 0),
        &agent.traits,
        &buildings,
        0,
    );
    let happiness_alone = total_happiness_modifier(&effects_alone);

    // With 3 other believers nearby
    let effects_community = calculate_religious_effects(
        Position::new(0, 0),
        &agent.traits,
        &buildings,
        3,
    );
    let happiness_community = total_happiness_modifier(&effects_community);

    // Community should provide more happiness
    assert!(happiness_community > happiness_alone);
}

#[test]
fn test_non_religious_agent_unaffected() {
    let agent = Agent::new(AgentConfig::default());
    // No religious traits

    let buildings = vec![
        (Position::new(0, 0), BuildingType::Temple, true),
        (Position::new(5, 5), BuildingType::Shrine, true),
    ];

    let effects = calculate_religious_effects(
        Position::new(0, 0),
        &agent.traits,
        &buildings,
        5, // Even with believers nearby
    );

    // No effects for non-religious agents
    assert!(effects.is_empty());
}

#[test]
fn test_distance_reduces_effect() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Believer);

    let buildings = vec![
        (Position::new(0, 0), BuildingType::Shrine, true),
    ];

    // At the shrine
    let effects_at = calculate_religious_effects(
        Position::new(0, 0),
        &agent.traits,
        &buildings,
        0,
    );
    let happiness_at = total_happiness_modifier(&effects_at);

    // 5 tiles away
    let effects_mid = calculate_religious_effects(
        Position::new(5, 0),
        &agent.traits,
        &buildings,
        0,
    );
    let happiness_mid = total_happiness_modifier(&effects_mid);

    // 9 tiles away (just within RELIGIOUS_EFFECT_RADIUS of 10)
    let effects_far = calculate_religious_effects(
        Position::new(9, 0),
        &agent.traits,
        &buildings,
        0,
    );
    let happiness_far = total_happiness_modifier(&effects_far);

    // Closer = stronger effect
    assert!(happiness_at > happiness_mid);
    assert!(happiness_mid > happiness_far);
}

#[test]
fn test_outside_radius_no_effect() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Believer);

    let buildings = vec![
        (Position::new(0, 0), BuildingType::Shrine, true),
    ];

    // Beyond RELIGIOUS_EFFECT_RADIUS
    let effects = calculate_religious_effects(
        Position::new((RELIGIOUS_EFFECT_RADIUS + 1) as i32, 0),
        &agent.traits,
        &buildings,
        0,
    );

    // No effects outside radius
    assert!(effects.is_empty());
}

#[test]
fn test_incomplete_building_no_effect() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Believer);

    let buildings = vec![
        (Position::new(0, 0), BuildingType::Temple, false), // Not completed
    ];

    let effects = calculate_religious_effects(
        Position::new(0, 0),
        &agent.traits,
        &buildings,
        0,
    );

    // Incomplete buildings don't affect agents
    assert!(effects.is_empty());
}

#[test]
fn test_should_seek_religious_building() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Believer);

    // Believer with low happiness should seek religious buildings
    assert!(should_seek_religious_building(&agent.traits, 0.4));

    // Believer with high happiness shouldn't
    assert!(!should_seek_religious_building(&agent.traits, 0.8));
}

#[test]
fn test_zealot_seeks_more_aggressively() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Zealot);

    // Zealot seeks even at moderate happiness (below 0.8)
    assert!(should_seek_religious_building(&agent.traits, 0.7));

    // Only when very happy do they stop seeking
    assert!(!should_seek_religious_building(&agent.traits, 0.9));
}

#[test]
fn test_atheist_avoids_religious_buildings() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Atheist);

    assert!(should_avoid_religious_building(&agent.traits));
}

#[test]
fn test_apply_religious_happiness_positive() {
    let mut agent = Agent::new(AgentConfig::default());
    let initial_happiness = agent.emotions.happiness;

    agent.apply_religious_happiness(0.2, "Temple blessing");

    // Happiness should increase from the religious effect
    assert!(agent.emotions.happiness > initial_happiness);
    // And should include the 0.2 added
    assert!(agent.emotions.happiness >= 0.2);
}

#[test]
fn test_apply_religious_happiness_negative() {
    use crate::agents::emotions::EmotionSource;

    let mut agent = Agent::new(AgentConfig::default());
    // Add some initial happiness via the source system
    agent.emotions.add_happiness(EmotionSource::Event("baseline".to_string()), 0.5);
    let initial_happiness = agent.emotions.happiness;

    agent.apply_religious_happiness(-0.15, "Temple discomfort");

    // Happiness should decrease (but not below 0)
    assert!(agent.emotions.happiness < initial_happiness);
    assert!(agent.emotions.happiness >= 0.0);
}

#[test]
fn test_multiple_religious_buildings_stack() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Believer);

    // Single shrine
    let single_building = vec![
        (Position::new(0, 0), BuildingType::Shrine, true),
    ];

    let effects_single = calculate_religious_effects(
        Position::new(0, 0),
        &agent.traits,
        &single_building,
        0,
    );
    let happiness_single = total_happiness_modifier(&effects_single);

    // Multiple shrines nearby
    let multiple_buildings = vec![
        (Position::new(0, 0), BuildingType::Shrine, true),
        (Position::new(3, 0), BuildingType::Shrine, true),
    ];

    let effects_multiple = calculate_religious_effects(
        Position::new(0, 0),
        &agent.traits,
        &multiple_buildings,
        0,
    );
    let happiness_multiple = total_happiness_modifier(&effects_multiple);

    // Multiple buildings should stack
    assert!(happiness_multiple > happiness_single);
}
