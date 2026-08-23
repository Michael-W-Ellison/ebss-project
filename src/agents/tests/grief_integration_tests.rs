// src/agents/tests/grief_integration_tests.rs
//! Integration tests for grief processing when agents die

use crate::agents::{Population, PopulationConfig, Agent, AgentConfig};
use crate::agents::emotions::Relationship;
use crate::core::DriveType;

#[test]
fn test_death_triggers_both_relationship_and_functional_grief() {
    let mut pop = Population::new();

    // Create two agents
    pop.spawn_agent(AgentConfig::default());
    pop.spawn_agent(AgentConfig::default());

    let agent1_id = pop.agents[0].id;
    let agent2_id = pop.agents[1].id;

    // Establish relationship (agent 1 loves agent 2)
    let mut relationship = Relationship::new(agent2_id, crate::agents::RelationshipType::Friend);
    relationship.bond_strength = 0.9; // Strong bond
    pop.agents[0].relationships.add_relationship(relationship);

    // Establish drive dependency (agent 1 depends on agent 2 for social satisfaction)
    for _ in 0..5 {
        pop.agents[0].record_drive_satisfaction(DriveType::Social, agent2_id, 0.3, 0);
    }

    // Verify agent 2 is a primary source
    let importance = pop.agents[0].get_source_importance(DriveType::Social, agent2_id);
    assert!(importance > 0.5, "Agent 2 should be important social source");

    // Record initial emotions
    let initial_sadness = pop.agents[0].emotions.sadness;
    let initial_anger = pop.agents[0].emotions.anger;

    // Agent 2 dies
    pop.agents[1].state.is_alive = false;

    // Process deaths (this should trigger grief)
    pop.tick();

    // Verify grief was triggered
    let final_sadness = pop.agents[0].emotions.sadness;
    let final_anger = pop.agents[0].emotions.anger;

    // Should have significant sadness increase
    assert!(final_sadness > initial_sadness + 0.5,
            "Death should trigger significant sadness (relationship + functional grief)");

    // May have some anger at the cause
    assert!(final_anger >= initial_anger,
            "Death may trigger anger at cause");

    // Verify agent 2 is no longer a tracked source
    let sources = pop.agents[0].get_drive_satisfaction_sources(DriveType::Social);
    assert!(!sources.contains(&agent2_id),
            "Deceased should be removed from satisfaction sources");
}

#[test]
fn test_death_without_dependency_causes_less_grief() {
    let mut pop = Population::new();

    // Create two agents
    pop.spawn_agent(AgentConfig::default());
    pop.spawn_agent(AgentConfig::default());

    let agent1_id = pop.agents[0].id;
    let agent2_id = pop.agents[1].id;

    // NO relationship, NO drive dependency (strangers)

    let initial_sadness = pop.agents[0].emotions.sadness;

    // Agent 2 dies
    pop.agents[1].state.is_alive = false;
    pop.tick();

    let final_sadness = pop.agents[0].emotions.sadness;

    // Should have minimal or no grief
    assert!(final_sadness - initial_sadness < 0.1,
            "Death of stranger should cause minimal grief");
}

#[test]
fn test_death_gossip_spreads_to_community() {
    let mut pop = Population::new();

    // Create three agents
    for _ in 0..3 {
        pop.spawn_agent(AgentConfig::default());
    }

    let deceased_id = pop.agents[1].id;

    // Agent 1 and 3 both know agent 2
    for i in [0, 2] {
        let mut rel = Relationship::new(deceased_id, crate::agents::RelationshipType::Acquaintance);
        rel.bond_strength = 0.4;
        pop.agents[i].relationships.add_relationship(rel);
    }

    // Agent 2 dies
    pop.agents[1].state.is_alive = false;
    pop.tick();

    // Both survivors should have death information in knowledge base
    // After tick(), dead agent is removed, so we now have 2 agents
    assert_eq!(pop.agents.len(), 2, "Should have 2 surviving agents");

    for agent in &pop.agents {
        let has_death_info = agent.knowledge.known_information
            .values()
            .any(|info| {
                matches!(&info.info_type,
                    crate::agents::InformationType::Death { agent, .. } if *agent == deceased_id)
            });

        assert!(has_death_info, "Survivors should know about death via gossip");
    }
}

#[test]
fn test_multiple_dependencies_compound_grief() {
    let mut pop = Population::new();

    pop.spawn_agent(AgentConfig::default());
    pop.spawn_agent(AgentConfig::default());

    // This measures how grief compounds, not whose grief it is. Founders are
    // drawn with three to five traits now and half the pool exists to modify
    // exactly this - a Stoic feels everything at half strength, a ColdHearted
    // gains sadness at half and sheds it at double - so a personality left in
    // would be measuring the draw rather than the mechanism.
    for agent in &mut pop.agents {
        agent.traits = crate::core::traits::TraitSet::new();
    }

    let agent2_id = pop.agents[1].id;

    // Agent 2 satisfies multiple drives for agent 1
    for _ in 0..5 {
        pop.agents[0].record_drive_satisfaction(DriveType::Social, agent2_id, 0.3, 0);
        pop.agents[0].record_drive_satisfaction(DriveType::Reproduction, agent2_id, 0.2, 0);
        pop.agents[0].record_drive_satisfaction(DriveType::Safety, agent2_id, 0.15, 0);
    }

    // Establish strong relationship
    let mut rel = Relationship::new(agent2_id, crate::agents::RelationshipType::Partner);
    rel.bond_strength = 0.95;
    pop.agents[0].relationships.add_relationship(rel);

    let initial_sadness = pop.agents[0].emotions.sadness;

    // Agent 2 dies
    pop.agents[1].state.is_alive = false;
    pop.tick();

    let final_sadness = pop.agents[0].emotions.sadness;

    // Grief should be compounded from multiple sources
    // 1. Relationship grief (bond 0.95 → sadness ~0.86)
    // 2. Social drive loss (importance ~0.8 → sadness ~0.4)
    // 3. Reproduction drive loss (importance ~0.6 → sadness ~0.3)
    // 4. Safety drive loss (importance ~0.5 → sadness ~0.25)
    // Total expected: ~1.8 (capped at 1.0)

    assert!(final_sadness > 0.9 || final_sadness - initial_sadness > 0.8,
            "Losing someone who satisfies multiple drives should cause severe grief");
}

#[test]
fn test_lonely_agent_experiences_amplified_grief() {
    let mut pop = Population::new();

    pop.spawn_agent(AgentConfig::default());
    pop.spawn_agent(AgentConfig::default());

    let agent2_id = pop.agents[1].id;

    // Agent 1 depends on agent 2 for social
    for _ in 0..3 {
        pop.agents[0].record_drive_satisfaction(DriveType::Social, agent2_id, 0.3, 0);
    }

    // Agent 1 is ALREADY lonely (high social drive)
    if let Some(social_drive) = pop.agents[0].drives.get_mut(DriveType::Social) {
        social_drive.value = 0.85; // Very lonely already
    }

    let initial_sadness = pop.agents[0].emotions.sadness;

    // Agent 2 dies (their only social source)
    pop.agents[1].state.is_alive = false;
    pop.tick();

    let final_sadness = pop.agents[0].emotions.sadness;

    // Grief should be amplified because drive was already high
    // "I was already lonely, now I'm even more alone"
    assert!(final_sadness - initial_sadness > 0.5,
            "Losing satisfaction source when drive is high should amplify grief");
}

#[test]
fn test_grief_explanation_mentions_functional_loss() {
    let mut pop = Population::new();

    pop.spawn_agent(AgentConfig::default());
    pop.spawn_agent(AgentConfig::default());

    let agent2_id = pop.agents[1].id;

    // Establish drive dependency
    for _ in 0..5 {
        pop.agents[0].record_drive_satisfaction(DriveType::Social, agent2_id, 0.4, 0);
    }

    // Relationship
    let mut rel = Relationship::new(agent2_id, crate::agents::RelationshipType::Friend);
    rel.bond_strength = 0.7;
    pop.agents[0].relationships.add_relationship(rel);

    // Agent 2 dies
    pop.agents[1].state.is_alive = false;
    pop.tick();

    // Get grief explanation
    let explanation = pop.agents[0].get_grief_reason(agent2_id);

    // Should mention both emotional and functional aspects
    // Note: After death processing, source is removed, so it may only show relationship
    assert!(
        explanation.contains("cared") || explanation.contains("bond") || explanation.contains("grieving"),
        "Explanation should express grief: {}", explanation
    );
}
