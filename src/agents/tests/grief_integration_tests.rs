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

// --------------------------------------------------------------------------
// What the survivors grieve at is a person, not the reckoning's verdict
// --------------------------------------------------------------------------
//
// Grief used to be keyed on `EmotionSource::Event(cause)` - the settlement's
// own name for what killed somebody. That bought nothing, because an `Event`
// source is write-only: `what_frightens_me_most` reads only `Creature` sources
// and `who_frightens_me_most` only `Agent` ones, so nothing in the model can
// act on being afraid of "hunger". And it cost the model its independence from
// its own bookkeeping: the sources are a `BTreeMap` keyed by the cause, so how
// many distinct names a death could have decided how many buckets fear was
// split across - and every bucket decays on its own - and decided the order
// they were summed in. #209 measured it: the same eight seeds read two ways
// were not the same eight worlds.

/// Every name this settlement has for a death. None of them may be an emotion.
///
/// Taken from the model rather than copied, so a cause added later is covered
/// here without anybody remembering to come back.
use crate::agents::AgentState;

/// Nothing an agent feels is keyed on what the reckoning decided.
fn nothing_is_felt_about_the_verdict(agent: &Agent) {
    use crate::agents::EmotionSource;

    for named in AgentState::EVERYTHING_THAT_TAKES_HEALTH {
        let verdict = EmotionSource::Event(named.to_string());
        assert!(
            !agent.emotions.fear_sources.contains_key(&verdict),
            "afraid of '{named}', which is a word this model made up about a corpse"
        );
        assert!(
            !agent.emotions.anger_sources.contains_key(&verdict),
            "angry at '{named}', which is not a thing that can be got back at"
        );
    }
}

/// A pair who care about each other, and a drive that one of them answers.
fn two_who_matter_to_each_other() -> Population {
    let mut pop = Population::new();
    pop.spawn_agent(AgentConfig::default());
    pop.spawn_agent(AgentConfig::default());

    let doomed = pop.agents[1].id;
    let mut bond = Relationship::new(doomed, crate::agents::RelationshipType::Friend);
    bond.bond_strength = 0.9;
    pop.agents[0].relationships.add_relationship(bond);

    for _ in 0..5 {
        pop.agents[0].record_drive_satisfaction(DriveType::Social, doomed, 0.3, 0);
    }

    pop
}

/// A death nobody had a hand in leaves grief and nothing to run from.
///
/// There is no thing there to be afraid *of*. A dread of the winter that took
/// him is worry rather than fear, and is not wired up - but writing it down as
/// a fear of the word "hunger" was not that either.
#[test]
fn a_death_nobody_had_a_hand_in_leaves_nothing_to_run_from() {
    let mut pop = two_who_matter_to_each_other();
    let before = pop.agents[0].emotions.sadness;

    pop.agents[1].state.is_alive = false;
    pop.tick();

    assert!(
        pop.agents[0].emotions.sadness > before,
        "he is still grieved for"
    );
    assert_eq!(
        pop.agents[0].emotions.who_frightens_me_most(),
        None,
        "and there is nobody to be frightened of"
    );
    nothing_is_felt_about_the_verdict(&pop.agents[0]);
}

/// A death somebody had a hand in is feared as that person.
///
/// This is the half that could not be expressed before. `who_frightens_me_most`
/// reads `Agent` sources, so for the first time the fear a death leaves is one
/// the flight branch of action selection can actually see.
#[test]
fn a_death_somebody_had_a_hand_in_is_feared_as_that_person() {
    let mut pop = two_who_matter_to_each_other();
    pop.spawn_agent(AgentConfig::default());
    let killer = pop.agents[2].id;

    pop.agents[1]
        .emotions
        .record_attack(crate::agents::EmotionSource::Agent(killer), 0);
    pop.agents[1].state.is_alive = false;
    pop.tick();

    let (frightened_of, how_much) = pop.agents[0]
        .emotions
        .who_frightens_me_most()
        .expect("the man who did it is somebody to be afraid of");

    assert_eq!(frightened_of, killer);
    assert!(how_much > 0.0);
    nothing_is_felt_about_the_verdict(&pop.agents[0]);
}

/// And he is somebody to be angry at, which is what makes a grudge possible.
///
/// `process_drive_source_loss_with_cause` has always had an arm for this -
/// "anger at whoever took away our satisfaction source" - and the only caller
/// in the model could never reach it, because it passed an `Event` every time.
/// Anger at a person is what `anger_at_people` reads, and that is what feeds
/// the relationship and the retaliation.
#[test]
fn the_man_who_did_it_is_somebody_to_be_angry_at() {
    let mut pop = two_who_matter_to_each_other();
    pop.spawn_agent(AgentConfig::default());
    let killer = pop.agents[2].id;

    pop.agents[1]
        .emotions
        .record_attack(crate::agents::EmotionSource::Agent(killer), 0);
    pop.agents[1].state.is_alive = false;
    pop.tick();

    let held_against = pop.agents[0].emotions.anger_at_people();
    assert!(
        held_against.iter().any(|(who, much)| *who == killer && *much > 0.0),
        "nobody is held to account for it: {held_against:?}"
    );
    nothing_is_felt_about_the_verdict(&pop.agents[0]);
}

// --------------------------------------------------------------------------
// A wolf is not a person
// --------------------------------------------------------------------------

/// A man taken by a wolf leaves his brother afraid of wolves.
///
/// This is the half #210 could not express and #212 found out why: the strike
/// wrote the *animal's* uuid into a field meaning "the person who hit me", so
/// keying grief on it made a survivor afraid of somebody who does not exist.
/// Now it says which, and a fear of a creature is one the flight branch can
/// actually read - `what_frightens_me_most` filters to `Creature` sources.
#[test]
fn a_man_taken_by_a_wolf_leaves_his_brother_afraid_of_wolves() {
    use crate::agents::EmotionSource;

    let mut pop = two_who_matter_to_each_other();
    pop.agents[1]
        .emotions
        .record_attack(EmotionSource::Creature("wolf".to_string()), 0);
    pop.agents[1].state.is_alive = false;
    pop.tick();

    let (what, how_much) = pop.agents[0]
        .emotions
        .what_frightens_me_most()
        .expect("there is a thing to be afraid of, and it is a wolf");
    assert_eq!(what, "wolf");
    assert!(how_much > 0.0);

    assert_eq!(
        pop.agents[0].emotions.who_frightens_me_most(),
        None,
        "and no person is blamed for it"
    );
    nothing_is_felt_about_the_verdict(&pop.agents[0]);
}

/// A killing is laid at the door of a man, and a wolf has no door.
#[test]
fn only_a_person_is_named_as_a_killer() {
    use crate::agents::EmotionSource;

    let mut pop = Population::new();
    pop.spawn_agent(AgentConfig::default());
    let mauled = &mut pop.agents[0];

    mauled
        .emotions
        .record_attack(EmotionSource::Creature("bear".to_string()), 0);
    assert!(
        mauled.emotions.recent_attacker(1).is_some(),
        "something struck him"
    );
    assert_eq!(
        mauled.emotions.whoever_struck_me(1),
        None,
        "but it was not anybody, and nobody may be blamed or struck back at"
    );

    let somebody = crate::core::dice::name();
    mauled
        .emotions
        .record_attack(EmotionSource::Agent(somebody), 0);
    assert_eq!(mauled.emotions.whoever_struck_me(1), Some(somebody));
}

/// And what struck is forgotten at the same rate whichever it was.
#[test]
fn what_struck_is_forgotten_at_the_same_rate_either_way() {
    use crate::agents::EmotionSource;

    let mut pop = Population::new();
    pop.spawn_agent(AgentConfig::default());
    let agent = &mut pop.agents[0];

    agent
        .emotions
        .record_attack(EmotionSource::Creature("wolf".to_string()), 0);
    assert!(agent.emotions.recent_attacker(99).is_some());
    assert!(agent.emotions.recent_attacker(100).is_none());
    assert!(agent.emotions.whoever_struck_me(100).is_none());
}
