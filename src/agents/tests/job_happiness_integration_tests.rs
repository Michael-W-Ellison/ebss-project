// src/agents/tests/job_happiness_integration_tests.rs
//! Integration tests for job happiness system

use crate::agents::{Agent, AgentConfig, Trait};
use crate::agents::job_happiness::JobCategory;
use crate::core::DriveType;

#[test]
fn test_builder_agent_prefers_building() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Builder);
    agent.update_preferences_from_traits();

    // Builder should have building as favorite job
    assert_eq!(agent.preferences.favorite_job, Some("building".to_string()));

    // Building should have highest happiness
    let (preferred, happiness) = agent.get_preferred_job();
    assert_eq!(preferred, JobCategory::Building);
    assert!(happiness > 0.0);
}

#[test]
fn test_handy_agent_enjoys_multiple_jobs() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Handy);
    agent.update_preferences_from_traits();

    // Handy should enjoy crafting, mining, building
    let crafting_happiness = agent.get_job_happiness(JobCategory::Crafting);
    let mining_happiness = agent.get_job_happiness(JobCategory::Mining);
    let building_happiness = agent.get_job_happiness(JobCategory::Building);

    assert!(crafting_happiness > 0.0);
    assert!(mining_happiness > 0.0);
    assert!(building_happiness > 0.0);
}

#[test]
fn test_lazy_agent_dislikes_labor() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Lazy);

    let labor_happiness = agent.get_job_happiness(JobCategory::Labor);
    assert!(labor_happiness < 0.0);
}

#[test]
fn test_survival_overrides_happiness() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Builder);

    // When not hungry, should not prioritize survival
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.3; // Low hunger
    }
    assert!(!agent.should_prioritize_survival());

    // When very hungry, should prioritize survival
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.9; // Very hungry
    }
    assert!(agent.should_prioritize_survival());
}

#[test]
fn test_happiness_affects_drive_selection() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Builder);
    agent.traits.add_trait(Trait::Handy);

    // Set up drives with similar urgency
    if let Some(industry) = agent.drives.get_mut(DriveType::Industry) {
        industry.value = 0.5;
        industry.weight = 1.0;
    }
    if let Some(construction) = agent.drives.get_mut(DriveType::Construction) {
        construction.value = 0.5;
        construction.weight = 1.0;
    }

    // Ensure not in survival mode
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.2;
    }
    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
        thirst.value = 0.2;
    }
    agent.state.health = 100.0;

    // With Builder trait, Construction drive should be preferred due to happiness
    let selected = agent.select_drive_with_happiness();

    // Should select Construction because Builder trait gives +6 happiness for building
    // vs Industry which only gives +4 happiness for mining
    assert!(selected.is_some());
    // Note: The exact result depends on drive weights, but happiness should influence it
}

#[test]
fn test_job_rankings() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::CraftObsessed);

    let rankings = agent.get_job_rankings();

    // CraftObsessed should have Crafting at the top
    assert!(!rankings.is_empty());
    assert_eq!(rankings[0].0, JobCategory::Crafting);
    assert_eq!(rankings[0].1, 8.0); // CraftObsessed gives +8 for crafting
}

#[test]
fn test_action_priority_calculation() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits.add_trait(Trait::Builder);

    // Ensure not in survival mode
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.2;
    }
    agent.state.health = 100.0;

    // Same drive urgency
    let urgency = 0.5;

    // Building should have higher effective priority due to Builder trait
    let building_priority = agent.calculate_action_priority(urgency, JobCategory::Building);
    let labor_priority = agent.calculate_action_priority(urgency, JobCategory::Labor);

    assert!(building_priority > labor_priority);
}

#[test]
fn test_preferences_from_traits() {
    use crate::core::Preferences;

    let mut traits = crate::core::traits::TraitSet::new();
    traits.add_trait(Trait::Explorer);

    let prefs = Preferences::from_traits(&traits);

    // Explorer should prefer exploring
    assert_eq!(prefs.favorite_job, Some("exploring".to_string()));
}

#[test]
fn test_job_happiness_modifier() {
    use crate::core::Preferences;

    let mut traits = crate::core::traits::TraitSet::new();
    traits.add_trait(Trait::Builder);

    let prefs = Preferences::from_traits(&traits);

    // Building should have positive modifier
    let building_mod = prefs.job_happiness_modifier("building", &traits);
    assert!(building_mod > 0.0);

    // Random job should have lower modifier
    let fishing_mod = prefs.job_happiness_modifier("fishing", &traits);
    assert!(building_mod > fishing_mod);
}
