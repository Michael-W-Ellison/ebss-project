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

    // Somebody who has been asking for food for days and not getting it. It is
    // the going without that frightens, not the wanting: a drive can sit near
    // its threshold all day while being answered every time it asks, and that
    // is not being prevented from anything.
    if let Some(hunger_drive) = agent.drives.get_mut(DriveType::Hunger) {
        hunger_drive.value = 0.95;
        hunger_drive.denied_ticks = 400;
    }
    agent.state.gone_without_food_for(21_600);

    agent.update_emotions_from_drives();

    let fear = agent.emotions.fear;
    assert!(fear > 0.0, "Being kept from food should cause fear");
    assert!(fear > 0.4, "Days of it should cause a good deal");
}

/// And wanting a thing that keeps arriving is not frightening.
#[test]
fn a_need_that_keeps_being_met_does_not_frighten_anybody() {
    let mut agent = Agent::new(AgentConfig::default());

    // High, but answered every time it asks, and the body in no trouble
    if let Some(hunger_drive) = agent.drives.get_mut(DriveType::Hunger) {
        hunger_drive.value = 0.95;
        hunger_drive.denied_ticks = 0;
    }
    agent.state.gone_without_food_for(0);

    agent.update_emotions_from_drives();

    assert!(
        agent.emotions.fear < 0.1,
        "somebody about to sit down to dinner is not afraid of anything; \
         fear stood at {:.2}",
        agent.emotions.fear
    );
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
    agent.record_drive_satisfaction(DriveType::Social, friend_id, 0.2, 0);

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
    agent.record_drive_satisfaction(DriveType::Social, friend1, 0.15, 0);
    agent.record_drive_satisfaction(DriveType::Social, friend2, 0.1, 0);
    agent.record_drive_satisfaction(DriveType::Social, family, 0.3, 0);

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
    agent.record_drive_satisfaction(DriveType::Social, best_friend, 0.4, 0);
    agent.record_drive_satisfaction(DriveType::Social, best_friend, 0.3, 0);
    agent.record_drive_satisfaction(DriveType::Social, best_friend, 0.35, 0);

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
    agent.record_drive_satisfaction(DriveType::Social, best_friend, 0.4, 0);
    agent.record_drive_satisfaction(DriveType::Social, best_friend, 0.35, 0);
    agent.record_drive_satisfaction(DriveType::Social, acquaintance, 0.05, 0);

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
        agent.record_drive_satisfaction(DriveType::Social, friend, 0.3, 0);
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
    agent.record_drive_satisfaction(DriveType::Social, friend, 0.4, 0);

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
    agent.record_drive_satisfaction(DriveType::Social, friend, 0.3, 0);

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

    // Kept from both, for days. It is the going without that frightens, not
    // the wanting: a drive can sit near its threshold all day while being
    // answered every time it asks, and that is not being prevented from
    // anything.
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.95;
        hunger.denied_ticks = 400;
    }
    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
        thirst.value = 0.9;
        thirst.denied_ticks = 400;
    }
    agent.state.gone_without_food_for(21_600);
    agent.state.gone_without_water_for(3_000);

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
        agent.record_drive_satisfaction(DriveType::Social, friend, 0.2, 0);
    }

    // Should recognize this as an important, reliable source
    let importance = agent.get_source_importance(DriveType::Social, friend);
    assert!(importance > 0.5, "Frequent satisfaction source should have high importance");

    // Compare to one-time source
    let stranger = Uuid::new_v4();
    agent.record_drive_satisfaction(DriveType::Social, stranger, 0.2, 0);

    let stranger_importance = agent.get_source_importance(DriveType::Social, stranger);
    assert!(importance > stranger_importance, "Frequent source should be more important than one-time");
}

#[test]
fn test_functional_grief_message() {
    let mut agent = Agent::new(AgentConfig::default());
    let friend = Uuid::new_v4();

    // Establish friend as social source
    for _ in 0..5 {
        agent.record_drive_satisfaction(DriveType::Social, friend, 0.3, 0);
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

    // Several needs going unanswered at once
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.85;
        hunger.denied_ticks = 400;
    }
    if let Some(social) = agent.drives.get_mut(DriveType::Social) {
        social.value = 0.8;
        social.denied_ticks = 400;
    }
    if let Some(rest) = agent.drives.get_mut(DriveType::Rest) {
        rest.value = 0.75;
        rest.denied_ticks = 400;
    }
    agent.state.gone_without_food_for(21_600);
    agent.state.energy = 10.0;

    agent.update_emotions_from_drives();

    // Combined frustration should be worse than single drive
    let total_negative = agent.emotions.fear + agent.emotions.sadness;
    assert!(total_negative > 0.65, "Multiple unmet drives should compound emotional distress");
}

// ===== Happiness System Tests =====

#[test]
fn test_satisfied_drives_create_happiness() {
    let mut agent = Agent::new(AgentConfig::default());
    
    // All drives well-satisfied (low values = satisfied)
    for drive in &mut agent.drives.drives {
        drive.value = 0.1; // Well satisfied
    }
    
    agent.update_emotions_from_drives();
    
    // Should have happiness from satisfied drives
    assert!(agent.emotions.happiness > 0.3, "Well-satisfied drives should create happiness, got: {}", agent.emotions.happiness);
}

#[test]
fn test_receiving_help_improves_bond() {
    let mut agent = Agent::new(AgentConfig::default());
    let helper = Uuid::new_v4();
    
    // Helper provides social satisfaction
    let initial_bond = if let Some(rel) = agent.relationships.get_relationship(&helper) {
        rel.bond_strength
    } else {
        0.0 // No relationship exists yet
    };
    
    agent.record_drive_satisfaction(DriveType::Social, helper, 0.4, 0);
    
    // Bond should improve
    let new_bond = agent.relationships.get_relationship(&helper).unwrap().bond_strength;
    assert!(new_bond > initial_bond, "Receiving help should improve bond");
    assert!(new_bond >= 0.2, "New bond should be at least 0.2");
}

#[test]
fn test_receiving_help_creates_happiness() {
    let mut agent = Agent::new(AgentConfig::default());
    let helper = Uuid::new_v4();
    
    // Helper provides help
    agent.record_drive_satisfaction(DriveType::Hunger, helper, 0.5, 0);
    
    // Should feel happiness (gratitude)
    assert!(agent.emotions.happiness > 0.1, "Receiving help should create happiness");
}

#[test]
fn test_helping_others_creates_happiness() {
    let mut agent = Agent::new(AgentConfig::default());
    let recipient = Uuid::new_v4();
    
    // Agent helps someone
    agent.process_helper_happiness(recipient, 0.4);
    
    // Should feel happiness from helping
    assert!(agent.emotions.happiness > 0.05, "Helping others should create happiness");
}

#[test]
fn test_empathetic_trait_bonus_for_helping() {
    use crate::core::traits::{Trait, TraitSet};
    
    let mut regular_agent = Agent::new(AgentConfig::default());
    let mut empathetic_agent = Agent::new(AgentConfig::default());
    empathetic_agent.traits.add_trait(Trait::Empathetic);
    
    let recipient = Uuid::new_v4();
    
    // Both help someone
    regular_agent.process_helper_happiness(recipient, 0.4);
    empathetic_agent.process_helper_happiness(recipient, 0.4);
    
    // Empathetic agent should feel more happiness
    assert!(empathetic_agent.emotions.happiness > regular_agent.emotions.happiness,
            "Empathetic agents should get bonus happiness from helping: empathetic={}, regular={}",
            empathetic_agent.emotions.happiness, regular_agent.emotions.happiness);
    assert!(empathetic_agent.emotions.happiness - regular_agent.emotions.happiness > 0.1,
            "Empathetic bonus should be significant");
}

#[test]
fn test_happiness_decays_over_time() {
    let mut agent = Agent::new(AgentConfig::default());
    let helper = Uuid::new_v4();
    
    // Receive help, creating happiness
    agent.record_drive_satisfaction(DriveType::Social, helper, 0.5, 0);
    let initial_happiness = agent.emotions.happiness;
    
    // Tick multiple times
    for _ in 0..50 {
        agent.emotions.tick();
    }
    
    // Happiness should decay
    assert!(agent.emotions.happiness < initial_happiness, 
            "Happiness should decay over time: initial={}, after_ticks={}", 
            initial_happiness, agent.emotions.happiness);
}

#[test]
fn test_well_being_considers_both_happiness_and_sadness() {
    let mut agent = Agent::new(AgentConfig::default());
    
    // High happiness
    use crate::agents::EmotionSource;
    agent.emotions.add_happiness(EmotionSource::Event("success".to_string()), 0.6);
    let happy_wellbeing = agent.emotions.well_being();
    
    // Now add sadness
    agent.emotions.add_sadness(EmotionSource::Event("loss".to_string()), 0.5);
    let mixed_wellbeing = agent.emotions.well_being();
    
    // Well-being should be lower with sadness
    assert!(mixed_wellbeing < happy_wellbeing, 
            "Negative emotions should reduce well-being");
    
    // Well-being should still be positive if happiness > sadness
    assert!(mixed_wellbeing > 0.0, 
            "Well-being should be positive when happiness outweighs sadness");
}
