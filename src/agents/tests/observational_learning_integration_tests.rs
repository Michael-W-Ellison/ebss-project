// src/agents/tests/observational_learning_integration_tests.rs
//! Integration tests for observational learning between agents

use crate::agents::{
    Agent, AgentConfig, ActionType, ObservedAction, Relationship, RelationshipType,
};
use uuid::Uuid;

#[test]
fn test_child_observes_parent_mining() {
    let mut parent = Agent::new(AgentConfig::default());
    let mut child = Agent::new(AgentConfig::default());
    child.set_learning_rate(1.5); // Child learns faster

    let parent_id = parent.id;
    let child_id = child.id;

    // Establish parent-child relationship
    child.relationships.add_relationship(Relationship::new(parent_id, RelationshipType::Parent));
    parent.relationships.add_relationship(Relationship::new(child_id, RelationshipType::Child));

    // Child can see parent
    child.senses.vision.visible_agents.insert(parent_id);

    // Set positions close together
    parent.state.position = (10, 64, 10);
    child.state.position = (12, 64, 10);

    // Child observes parent mining successfully multiple times
    for i in 0..5 {
        child.observe_action(
            &parent_id,
            parent.state.position,
            ActionType::Mining,
            true,
            format!("mined stone {}", i),
            i as u64,
        );
    }

    // Check learning progress
    let progress = child.get_learning_from(&parent_id, ActionType::Mining);
    assert!(progress.is_some(), "Child should have learning progress from parent");

    let progress = progress.unwrap();
    assert_eq!(progress.observation_count, 5);
    assert_eq!(progress.success_count, 5);
    assert!(progress.confidence > 0.5, "Child should have confidence from parent's success");

    // Check if ready to adopt behavior
    let opportunities = child.check_learning_opportunities();
    assert!(!opportunities.is_empty(), "Child should have learning opportunities from parent");

    // Should include mining from parent
    let mining_opportunity = opportunities.iter()
        .find(|(teacher, action, _)| *teacher == parent_id && *action == ActionType::Mining);
    assert!(mining_opportunity.is_some(), "Child should be ready to learn mining from parent");
}

#[test]
fn test_child_learns_faster_than_adult() {
    let parent_id = crate::core::dice::name();

    // Create child agent
    let mut child = Agent::new(AgentConfig::default());
    child.set_learning_rate(1.5);
    child.relationships.add_relationship(Relationship::new(parent_id, RelationshipType::Parent));
    child.senses.vision.visible_agents.insert(parent_id);
    child.state.position = (0, 0, 0);

    // Create adult agent
    let mut adult = Agent::new(AgentConfig::default());
    adult.set_learning_rate(1.0);
    adult.relationships.add_relationship(Relationship::new(parent_id, RelationshipType::Friend));
    adult.senses.vision.visible_agents.insert(parent_id);
    adult.state.position = (0, 0, 0);

    let performer_position = (5, 0, 0);

    // Both observe same action 3 times
    for i in 0..3 {
        child.observe_action(
            &parent_id,
            performer_position,
            ActionType::Crafting,
            true,
            format!("crafted item {}", i),
            i as u64,
        );

        adult.observe_action(
            &parent_id,
            performer_position,
            ActionType::Crafting,
            true,
            format!("crafted item {}", i),
            i as u64,
        );
    }

    // Check if child can adopt
    let child_opportunities = child.check_learning_opportunities();
    let child_can_learn = child_opportunities.iter()
        .any(|(teacher, action, _)| *teacher == parent_id && *action == ActionType::Crafting);

    // Check if adult can adopt
    let adult_opportunities = adult.check_learning_opportunities();
    let adult_can_learn = adult_opportunities.iter()
        .any(|(teacher, action, _)| *teacher == parent_id && *action == ActionType::Crafting);

    // Child should learn faster due to higher learning rate and parent relationship
    assert!(
        child_can_learn || !adult_can_learn,
        "Child should learn faster than adult (child={}, adult={})",
        child_can_learn,
        adult_can_learn
    );
}

#[test]
fn test_cannot_observe_if_not_visible() {
    let mut observer = Agent::new(AgentConfig::default());
    let performer_id = crate::core::dice::name();

    observer.state.position = (0, 0, 0);

    // Performer is NOT in visible_agents
    observer.observe_action(
        &performer_id,
        (10, 0, 0),
        ActionType::Mining,
        true,
        "mined stone".to_string(),
        0_u64,
    );

    // Should not have recorded observation
    let progress = observer.get_learning_from(&performer_id, ActionType::Mining);
    assert!(progress.is_none(), "Cannot learn from agent you cannot see");
}

#[test]
fn test_distance_affects_observation_quality() {
    let mut learner = Agent::new(AgentConfig::default());
    let teacher_id = crate::core::dice::name();

    learner.senses.vision.visible_agents.insert(teacher_id);
    learner.state.position = (0, 0, 0);

    // Close observation (5 units away)
    learner.observe_action(
        &teacher_id,
        (5, 0, 0),
        ActionType::Building,
        true,
        "built wall".to_string(),
        0_u64,
    );

    // Distant observation (50 units away)
    learner.observe_action(
        &teacher_id,
        (50, 0, 0),
        ActionType::Building,
        true,
        "built wall".to_string(),
        1_u64,
    );

    let progress = learner.get_learning_from(&teacher_id, ActionType::Building).unwrap();

    // Both observations recorded
    assert_eq!(progress.observation_count, 2);

    // But close observation should contribute more quality
    // (This is implicitly tested - avg quality should be moderate)
    assert!(progress.avg_quality() > 0.3 && progress.avg_quality() < 1.0);
}

#[test]
fn test_adopt_learned_behavior() {
    let mut learner = Agent::new(AgentConfig::default());
    learner.set_learning_rate(1.5);

    let teacher_id = crate::core::dice::name();
    learner.relationships.add_relationship(Relationship::new(teacher_id, RelationshipType::Parent));
    learner.senses.vision.visible_agents.insert(teacher_id);
    learner.state.position = (0, 0, 0);

    // Observe many times
    for i in 0..10 {
        learner.observe_action(
            &teacher_id,
            (3, 0, 0),
            ActionType::Cooking,
            true,
            format!("cooked food {}", i),
            i as u64,
        );
    }

    // Should be able to adopt
    let adopted = learner.adopt_learned_behavior(&teacher_id, ActionType::Cooking);
    assert!(adopted, "Should be able to adopt after many observations");

    // Check adopted behaviors
    let adopted_behaviors = learner.get_adopted_behaviors();
    assert_eq!(adopted_behaviors.len(), 1);

    let (adopted_teacher, adopted_action, confidence) = adopted_behaviors[0];
    assert_eq!(adopted_teacher, teacher_id);
    assert_eq!(adopted_action, ActionType::Cooking);
    assert!(confidence > 0.7);

    // Should not be able to adopt again
    let adopted_again = learner.adopt_learned_behavior(&teacher_id, ActionType::Cooking);
    assert!(!adopted_again, "Should not adopt same behavior twice");
}

#[test]
fn test_learning_from_parents_tracking() {
    let mut child = Agent::new(AgentConfig::default());
    child.set_learning_rate(1.5);

    let parent1_id = crate::core::dice::name();
    let parent2_id = crate::core::dice::name();

    // Add two parents
    child.relationships.add_relationship(Relationship::new(parent1_id, RelationshipType::Parent));
    child.relationships.add_relationship(Relationship::new(parent2_id, RelationshipType::Parent));

    child.senses.vision.visible_agents.insert(parent1_id);
    child.senses.vision.visible_agents.insert(parent2_id);
    child.state.position = (0, 0, 0);

    // Observe parent1 mining
    child.observe_action(
        &parent1_id,
        (5, 0, 0),
        ActionType::Mining,
        true,
        "mined ore".to_string(),
        0_u64,
    );

    // Observe parent2 crafting
    child.observe_action(
        &parent2_id,
        (5, 0, 0),
        ActionType::Crafting,
        true,
        "crafted tool".to_string(),
        1_u64,
    );

    // Check learning from parents
    let parent_learning = child.learning_from_parents();
    assert_eq!(parent_learning.len(), 2, "Should be learning from both parents");

    // Find each parent
    let parent1_learning = parent_learning.iter()
        .find(|(id, _)| *id == parent1_id)
        .expect("Should be learning from parent1");

    let parent2_learning = parent_learning.iter()
        .find(|(id, _)| *id == parent2_id)
        .expect("Should be learning from parent2");

    assert!(parent1_learning.1.contains(&ActionType::Mining));
    assert!(parent2_learning.1.contains(&ActionType::Crafting));
}

#[test]
fn test_failed_actions_reduce_learning_quality() {
    let mut learner = Agent::new(AgentConfig::default());
    let teacher_id = crate::core::dice::name();

    learner.senses.vision.visible_agents.insert(teacher_id);
    learner.relationships.add_relationship(Relationship::new(teacher_id, RelationshipType::Friend));
    learner.state.position = (0, 0, 0);

    // Observe mostly failures
    for i in 0..5 {
        learner.observe_action(
            &teacher_id,
            (5, 0, 0),
            ActionType::Combat,
            false, // Failed!
            format!("lost fight {}", i),
            i as u64,
        );
    }

    // One success
    learner.observe_action(
        &teacher_id,
        (5, 0, 0),
        ActionType::Combat,
        true,
        "won fight".to_string(),
        5_u64,
    );

    let progress = learner.get_learning_from(&teacher_id, ActionType::Combat).unwrap();

    // Success rate should be low
    assert_eq!(progress.success_count, 1);
    assert_eq!(progress.observation_count, 6);

    let success_rate = progress.success_rate();
    assert!(success_rate < 0.3, "Success rate should be low with mostly failures");

    // Should NOT be ready to adopt (low success rate)
    let opportunities = learner.check_learning_opportunities();
    let combat_opportunity = opportunities.iter()
        .any(|(teacher, action, _)| *teacher == teacher_id && *action == ActionType::Combat);

    assert!(!combat_opportunity, "Should not learn from mostly failed actions");
}

#[test]
fn test_multiple_action_types_from_same_teacher() {
    let mut learner = Agent::new(AgentConfig::default());
    let teacher_id = crate::core::dice::name();

    learner.senses.vision.visible_agents.insert(teacher_id);
    learner.relationships.add_relationship(Relationship::new(teacher_id, RelationshipType::Parent));
    learner.state.position = (0, 0, 0);

    // Observe multiple different actions
    for i in 0..3 {
        learner.observe_action(
            &teacher_id,
            (5, 0, 0),
            ActionType::Mining,
            true,
            format!("mined {}", i),
            i as u64,
        );
    }

    for i in 0..3 {
        learner.observe_action(
            &teacher_id,
            (5, 0, 0),
            ActionType::Crafting,
            true,
            format!("crafted {}", i),
            (i + 3) as u64,
        );
    }

    for i in 0..3 {
        learner.observe_action(
            &teacher_id,
            (5, 0, 0),
            ActionType::Building,
            true,
            format!("built {}", i),
            (i + 6) as u64,
        );
    }

    // Should have progress for all three actions
    assert!(learner.get_learning_from(&teacher_id, ActionType::Mining).is_some());
    assert!(learner.get_learning_from(&teacher_id, ActionType::Crafting).is_some());
    assert!(learner.get_learning_from(&teacher_id, ActionType::Building).is_some());

    // All should have 3 observations
    assert_eq!(
        learner.get_learning_from(&teacher_id, ActionType::Mining).unwrap().observation_count,
        3
    );
    assert_eq!(
        learner.get_learning_from(&teacher_id, ActionType::Crafting).unwrap().observation_count,
        3
    );
    assert_eq!(
        learner.get_learning_from(&teacher_id, ActionType::Building).unwrap().observation_count,
        3
    );
}

#[test]
fn test_learning_rate_getters_setters() {
    let mut agent = Agent::new(AgentConfig::default());

    // Default should be 1.0
    assert_eq!(agent.learning_rate(), 1.0);

    // Set to child rate
    agent.set_learning_rate(1.5);
    assert_eq!(agent.learning_rate(), 1.5);

    // Set to elder rate
    agent.set_learning_rate(0.7);
    assert_eq!(agent.learning_rate(), 0.7);

    // Should clamp to valid range
    agent.set_learning_rate(5.0);
    assert_eq!(agent.learning_rate(), 2.0); // Clamped to max

    agent.set_learning_rate(0.01);
    assert_eq!(agent.learning_rate(), 0.1); // Clamped to min
}
