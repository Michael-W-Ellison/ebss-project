// src/agents/tests/drive_emotion_feedback_tests.rs
//! TDD tests for drive-emotion feedback system
//!
//! These tests define how unsatisfied drives and loss of drive satisfaction sources
//! should trigger emotional responses, creating functional grief and drive frustration.

use crate::agents::{Agent, AgentConfig};
use crate::core::{DriveType, EmotionType};
use uuid::Uuid;

#[test]
fn test_high_social_drive_causes_sadness() {
    let mut agent = Agent::new(AgentConfig::default());

    // Manually set social drive to critically high
    if let Some(social_drive) = agent.drives.get_mut(DriveType::Social) {
        social_drive.value = 0.9; // Very lonely
    }

    // Update emotions based on drives
    agent.update_emotions_from_drives();

    // Should feel sadness from unmet social need
    let sadness = agent.emotions.sadness;
    assert!(sadness > 0.0, "High social drive should cause sadness");
    assert!(sadness > 0.3, "Critically high social drive should cause significant sadness");
}

#[test]
fn test_high_hunger_causes_fear() {
    let mut agent = Agent::new(AgentConfig::default());

    // Manually set hunger to critically high
    if let Some(hunger_drive) = agent.drives.get_mut(DriveType::Hunger) {
        hunger_drive.value = 0.95; // Starving
    }

    // Update emotions based on drives
    agent.update_emotions_from_drives();

    // Should feel fear from survival threat
    let fear = agent.emotions.fear;
    assert!(fear > 0.0, "Critical hunger should cause fear");
    assert!(fear > 0.4, "Near-starvation should cause significant fear");
}

#[test]
fn test_moderate_drive_no_strong_emotion() {
    let mut agent = Agent::new(AgentConfig::default());

    // Set drive to moderate level
    if let Some(social_drive) = agent.drives.get_mut(DriveType::Social) {
        social_drive.value = 0.4; // Moderate
    }

    agent.update_emotions_from_drives();

    // Should not trigger strong emotions
    assert!(agent.emotions.sadness < 0.2, "Moderate drives should not cause strong sadness");
    assert!(agent.emotions.fear < 0.2, "Moderate drives should not cause strong fear");
}

#[test]
fn test_satisfied_drive_no_negative_emotion() {
    let mut agent = Agent::new(AgentConfig::default());

    // All drives satisfied
    for drive in &mut agent.drives.drives {
        drive.value = 0.0;
    }

    agent.update_emotions_from_drives();

    // Should have minimal negative emotions
    assert!(agent.emotions.sadness < 0.1, "Satisfied drives should not cause sadness");
    assert!(agent.emotions.fear < 0.1, "Satisfied drives should not cause fear");
}

#[test]
fn test_track_social_satisfaction_source() {
    let mut agent = Agent::new(AgentConfig::default());
    let friend_id = Uuid::new_v4();

    // Record that friend satisfies social drive
    agent.record_drive_satisfaction(DriveType::Social, friend_id, 0.2);

    // Should track this source
    let sources = agent.get_drive_satisfaction_sources(DriveType::Social);
    assert_eq!(sources.len(), 1, "Should track social satisfaction source");
    assert!(sources.contains(&friend_id), "Should track friend as source");
}

#[test]
fn test_multiple_social_sources() {
    let mut agent = Agent::new(AgentConfig::default());
    let friend1 = Uuid::new_v4();
    let friend2 = Uuid::new_v4();
    let family = Uuid::new_v4();

    // Record multiple sources
    agent.record_drive_satisfaction(DriveType::Social, friend1, 0.15);
    agent.record_drive_satisfaction(DriveType::Social, friend2, 0.1);
    agent.record_drive_satisfaction(DriveType::Social, family, 0.3);

    let sources = agent.get_drive_satisfaction_sources(DriveType::Social);
    assert_eq!(sources.len(), 3, "Should track all social sources");

    // Should identify most important source
    let primary_source = agent.get_primary_satisfaction_source(DriveType::Social);
    assert_eq!(primary_source, Some(family), "Should identify family as primary social source");
}

#[test]
fn test_loss_of_social_source_triggers_sadness() {
    let mut agent = Agent::new(AgentConfig::default());
    let best_friend = Uuid::new_v4();

    // Establish friend as primary social source
    agent.record_drive_satisfaction(DriveType::Social, best_friend, 0.4);
    agent.record_drive_satisfaction(DriveType::Social, best_friend, 0.3);
    agent.record_drive_satisfaction(DriveType::Social, best_friend, 0.35);

    // Lose the friend
    let initial_sadness = agent.emotions.sadness;
    agent.process_drive_source_loss(DriveType::Social, best_friend);

    // Should trigger sadness
    let sadness_increase = agent.emotions.sadness - initial_sadness;
    assert!(sadness_increase > 0.0, "Losing social source should increase sadness");
    assert!(sadness_increase > 0.3, "Losing primary social source should cause significant sadness");
}

#[test]
fn test_loss_of_minor_source_less_sadness() {
    let mut agent = Agent::new(AgentConfig::default());
    let best_friend = Uuid::new_v4();
    let acquaintance = Uuid::new_v4();

    // Establish friend as primary source, acquaintance as minor
    agent.record_drive_satisfaction(DriveType::Social, best_friend, 0.4);
    agent.record_drive_satisfaction(DriveType::Social, best_friend, 0.35);
    agent.record_drive_satisfaction(DriveType::Social, acquaintance, 0.05);

    // Lose the acquaintance
    let initial_sadness = agent.emotions.sadness;
    agent.process_drive_source_loss(DriveType::Social, acquaintance);

    let sadness_increase = agent.emotions.sadness - initial_sadness;
    assert!(sadness_increase < 0.2, "Losing minor source should cause less sadness");
}

#[test]
fn test_source_loss_with_high_drive_amplifies_emotion() {
    let mut agent = Agent::new(AgentConfig::default());
    let friend = Uuid::new_v4();

    // Friend was satisfying social drive (multiple interactions = important source)
    for _ in 0..3 {
        agent.record_drive_satisfaction(DriveType::Social, friend, 0.3);
    }

    // Social drive is now high (lonely)
    if let Some(social_drive) = agent.drives.get_mut(DriveType::Social) {
        social_drive.value = 0.8;
    }

    // Lose friend when already lonely
    let initial_sadness = agent.emotions.sadness;
    agent.process_drive_source_loss(DriveType::Social, friend);

    // Should cause extra sadness (functional grief: "I was already lonely, now I'm even more alone")
    let sadness_increase = agent.emotions.sadness - initial_sadness;
    assert!(sadness_increase > 0.4, "Losing source when drive is high should amplify sadness");
}

#[test]
fn test_anger_at_source_of_death_when_losing_satisfaction_source() {
    let mut agent = Agent::new(AgentConfig::default());
    let friend = Uuid::new_v4();
    let killer = Uuid::new_v4();

    // Friend satisfies social drive
    agent.record_drive_satisfaction(DriveType::Social, friend, 0.4);

    // Friend killed by another agent
    let initial_anger = agent.emotions.anger;
    agent.process_drive_source_loss_with_cause(
        DriveType::Social,
        friend,
        Some(crate::agents::EmotionSource::Agent(killer))
    );

    // Should feel anger at killer for removing satisfaction source
    let anger_increase = agent.emotions.anger - initial_anger;
    assert!(anger_increase > 0.0, "Should feel anger at cause of source loss");
    assert!(anger_increase > 0.2, "Losing important source should cause significant anger");
}

#[test]
fn test_no_anger_at_natural_death() {
    let mut agent = Agent::new(AgentConfig::default());
    let friend = Uuid::new_v4();

    // Friend satisfies social drive
    agent.record_drive_satisfaction(DriveType::Social, friend, 0.3);

    // Friend dies of natural causes
    let initial_anger = agent.emotions.anger;
    agent.process_drive_source_loss_with_cause(
        DriveType::Social,
        friend,
        Some(crate::agents::EmotionSource::Event("old age".to_string()))
    );

    // Should not feel anger (no one to blame)
    let anger_increase = agent.emotions.anger - initial_anger;
    assert!(anger_increase < 0.1, "Natural death should not cause significant anger");
}

#[test]
fn test_survival_drives_cause_fear_not_sadness() {
    let mut agent = Agent::new(AgentConfig::default());

    // Set survival drives critically high
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.95;
    }
    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
        thirst.value = 0.9;
    }

    agent.update_emotions_from_drives();

    // Survival threats should cause fear, not sadness
    assert!(agent.emotions.fear > 0.4, "Survival threats should cause fear");
    assert!(agent.emotions.sadness < agent.emotions.fear, "Fear should dominate over sadness for survival drives");
}

#[test]
fn test_social_drives_cause_sadness_not_fear() {
    let mut agent = Agent::new(AgentConfig::default());

    // Set social drive critically high
    if let Some(social) = agent.drives.get_mut(DriveType::Social) {
        social.value = 0.9;
    }

    agent.update_emotions_from_drives();

    // Social deprivation should cause sadness, not fear
    assert!(agent.emotions.sadness > 0.3, "Social deprivation should cause sadness");
    assert!(agent.emotions.sadness > agent.emotions.fear, "Sadness should dominate over fear for social drives");
}

#[test]
fn test_drive_frustration_decays_when_satisfied() {
    let mut agent = Agent::new(AgentConfig::default());

    // Drive is high, causing sadness
    if let Some(social) = agent.drives.get_mut(DriveType::Social) {
        social.value = 0.85;
    }
    agent.update_emotions_from_drives();
    let initial_sadness = agent.emotions.sadness;

    // Satisfy the drive
    if let Some(social) = agent.drives.get_mut(DriveType::Social) {
        social.value = 0.2;
    }
    agent.update_emotions_from_drives();

    // Sadness should decrease
    assert!(agent.emotions.sadness < initial_sadness, "Satisfying drive should reduce sadness");
}

#[test]
fn test_source_importance_tracked_over_time() {
    let mut agent = Agent::new(AgentConfig::default());
    let friend = Uuid::new_v4();

    // Record satisfaction over multiple interactions
    for _ in 0..10 {
        agent.record_drive_satisfaction(DriveType::Social, friend, 0.2);
    }

    // Should recognize this as an important, reliable source
    let importance = agent.get_source_importance(DriveType::Social, friend);
    assert!(importance > 0.5, "Frequent satisfaction source should have high importance");

    // Compare to one-time source
    let stranger = Uuid::new_v4();
    agent.record_drive_satisfaction(DriveType::Social, stranger, 0.2);

    let stranger_importance = agent.get_source_importance(DriveType::Social, stranger);
    assert!(importance > stranger_importance, "Frequent source should be more important than one-time");
}

#[test]
fn test_functional_grief_message() {
    let mut agent = Agent::new(AgentConfig::default());
    let friend = Uuid::new_v4();

    // Establish friend as social source
    for _ in 0..5 {
        agent.record_drive_satisfaction(DriveType::Social, friend, 0.3);
    }

    // Create relationship
    use crate::agents::emotions::{Relationship, RelationshipType};
    agent.relationships.add_relationship(Relationship::new(friend, RelationshipType::Friend));

    // Process loss
    agent.process_drive_source_loss(DriveType::Social, friend);

    // Should be able to explain grief functionally
    // Note: After processing loss, source is removed from tracker
    // But we still have the relationship, so grief reason should mention caring
    let grief_reason = agent.get_grief_reason(friend);
    assert!(grief_reason.contains("cared") || grief_reason.contains("bond") || grief_reason.contains("grieving"),
            "Grief explanation should express emotional connection, got: {}", grief_reason);
}

#[test]
fn test_multiple_drive_frustration_compounds() {
    let mut agent = Agent::new(AgentConfig::default());

    // Multiple high drives
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.85;
    }
    if let Some(social) = agent.drives.get_mut(DriveType::Social) {
        social.value = 0.8;
    }
    if let Some(rest) = agent.drives.get_mut(DriveType::Rest) {
        rest.value = 0.75;
    }

    agent.update_emotions_from_drives();

    // Combined frustration should be worse than single drive
    let total_negative = agent.emotions.fear + agent.emotions.sadness;
    assert!(total_negative > 0.65, "Multiple unmet drives should compound emotional distress");
}
