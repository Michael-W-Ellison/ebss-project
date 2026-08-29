// src/analytics/tests/distrust_tests.rs
//! Tests that whose word an agent takes depends on who is speaking, and that
//! what a lie costs depends on what was lied about.
//!
//! Trust was kept in three unconnected books - a verified track record in the
//! knowledge base, an enum on the relationship, and a sum of trait modifiers
//! that mixed "do I believe people" with "do people believe me" - and the
//! channel that actually carries information between agents consulted none of
//! them. Resource and building locations went straight into
//! `exploration_knowledge`, which is what foraging reads, from anybody at all,
//! and could not be wrong: `would_lie_to` weighed honesty and the relationship
//! and its only caller was itself never called, so no lie had ever been told
//! in a running settlement.
//!
//! And being lied to cost a flat 0.2 anger whatever the lie was about, so
//! sending a thirsty man to a dry riverbed and misdescribing a pile of rocks
//! came to exactly the same thing.

use crate::agents::{Agent, AgentConfig, LifeStage, Population, SkillType};
use crate::core::{DriveType, Trait};
use crate::world::{Position, ResourceType, World, WorldConfig};

fn somebody() -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits = crate::core::traits::TraitSet::new();
    agent.state.life_stage = LifeStage::Adult;
    agent.state.health = 100.0;
    agent
}

/// A friend is believed and an enemy is not.
#[test]
fn whose_word_you_take_depends_on_what_they_are_to_you() {
    let mut listener = somebody();
    let friend = somebody();
    let enemy = somebody();

    listener
        .relationships
        .get_or_create_relationship(friend.id, 0)
        .strengthen(0.8);
    listener
        .relationships
        .get_or_create_relationship(enemy.id, 0)
        .weaken(1.0);

    assert!(
        listener.would_take_their_word(friend.id, &friend.traits),
        "a friend's word is worth having"
    );
    assert!(
        !listener.would_take_their_word(enemy.id, &enemy.traits),
        "and a man you cannot stand is not worth listening to"
    );
}

/// Two people hear the same thing from the same man and only one believes it.
#[test]
fn what_sort_of_person_is_listening_decides_it_too() {
    let speaker = somebody();

    let mut trusting = somebody();
    trusting.traits.add_trait(Trait::Trusting);
    let mut paranoid = somebody();
    paranoid.traits.add_trait(Trait::Paranoid);

    // Neither of them knows the man from Adam
    assert!(
        trusting.how_far_i_trust(speaker.id, &speaker.traits)
            > paranoid.how_far_i_trust(speaker.id, &speaker.traits),
        "a trusting man and a paranoid one do not hear the same stranger the \
         same way"
    );
    assert!(!paranoid.would_take_their_word(speaker.id, &speaker.traits));
}

/// And what sort of person is talking.
#[test]
fn what_sort_of_person_is_talking_decides_it_as_well() {
    let listener = somebody();

    let plain = somebody();
    let mut charming = somebody();
    charming.traits.add_trait(Trait::Charismatic);
    let mut cruel = somebody();
    cruel.traits.add_trait(Trait::Cruel);

    let for_plain = listener.how_far_i_trust(plain.id, &plain.traits);
    assert!(
        listener.how_far_i_trust(charming.id, &charming.traits) > for_plain,
        "a charmer gets the benefit of the doubt"
    );
    assert!(
        listener.how_far_i_trust(cruel.id, &cruel.traits) < for_plain,
        "and somebody who puts you off does not"
    );
}

/// Being caught out costs a man the credit he had.
#[test]
fn being_caught_out_costs_a_man_his_credit() {
    let mut listener = somebody();
    let liar = somebody();

    // They were on ordinary terms
    listener
        .relationships
        .get_or_create_relationship(liar.id, 0)
        .strengthen(0.3);
    let before = listener.how_far_i_trust(liar.id, &liar.traits);

    for _ in 0..3 {
        listener.found_out_i_was_lied_to(liar.id, "food", 100);
    }

    assert!(
        listener.how_far_i_trust(liar.id, &liar.traits) < before,
        "three lies should tell: trust ran {before:.2} and now runs {:.2}",
        listener.how_far_i_trust(liar.id, &liar.traits)
    );
    assert!(
        !listener.would_take_their_word(liar.id, &liar.traits),
        "and his word should no longer be worth taking"
    );
}

/// A lie about what somebody needs costs more than a lie about what they do
/// not.
#[test]
fn a_lie_about_what_a_man_needs_costs_more() {
    let mut starving = somebody();
    starving.state.gone_without_food_for(9_600);
    if let Some(hunger) = starving.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.95;
        hunger.denied_ticks = 400;
    }

    let liar = uuid::Uuid::new_v4();

    let about_food = starving.what_a_lie_about_this_costs(Some("food"), liar);
    let about_stone = starving.what_a_lie_about_this_costs(Some("stone"), liar);

    assert!(
        about_food > about_stone,
        "sending a starving man to an empty field is not the same as \
         misdescribing a pile of rocks: {about_food:.2} against {about_stone:.2}"
    );
}

/// And the same lie costs less when the need is not pressing.
#[test]
fn the_same_lie_costs_less_to_a_man_who_is_not_hungry() {
    let liar = uuid::Uuid::new_v4();

    let mut starving = somebody();
    starving.state.gone_without_food_for(9_600);
    if let Some(hunger) = starving.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.95;
        hunger.denied_ticks = 400;
    }

    let mut fed = somebody();
    fed.state.gone_without_food_for(0);

    assert!(
        starving.what_a_lie_about_this_costs(Some("food"), liar)
            > fed.what_a_lie_about_this_costs(Some("food"), liar),
        "the same lie about food is a different thing to a man with a full \
         stomach"
    );
}

/// Being deceived by a friend is worse than by somebody you had no time for.
#[test]
fn a_lie_from_a_friend_cuts_deeper() {
    let liar = uuid::Uuid::new_v4();

    let mut trusted_him = somebody();
    trusted_him
        .relationships
        .get_or_create_relationship(liar, 0)
        .strengthen(0.6);

    let mut never_liked_him = somebody();
    never_liked_him
        .relationships
        .get_or_create_relationship(liar, 0)
        .weaken(0.5);

    assert!(
        trusted_him.what_a_lie_about_this_costs(Some("water"), liar)
            > never_liked_him.what_a_lie_about_this_costs(Some("water"), liar),
        "being deceived by somebody you trusted is worse than being deceived \
         by somebody you did not"
    );
}

/// A vengeful man holds it against you and a forgiving one does not.
#[test]
fn what_sort_of_person_was_lied_to_decides_what_it_costs() {
    let liar = uuid::Uuid::new_v4();

    let plain = somebody();
    let mut vengeful = somebody();
    vengeful.traits.add_trait(Trait::Vengeful);
    let mut forgiving = somebody();
    forgiving.traits.add_trait(Trait::Forgiving);

    let ordinary = plain.what_a_lie_about_this_costs(Some("water"), liar);

    assert!(vengeful.what_a_lie_about_this_costs(Some("water"), liar) > ordinary);
    assert!(forgiving.what_a_lie_about_this_costs(Some("water"), liar) < ordinary);
}

/// An honest man does not lie, whatever he thinks of you.
#[test]
fn an_honest_man_does_not_lie_even_to_somebody_he_cannot_stand() {
    let mut honest = somebody();
    honest.traits.add_trait(Trait::Honest);

    let them = uuid::Uuid::new_v4();
    honest
        .relationships
        .get_or_create_relationship(them, 0)
        .weaken(1.5);

    for _ in 0..50 {
        assert!(
            !honest.would_lie_to(them, 0),
            "not once in fifty times of asking"
        );
    }
}

/// Walking to the place is what finds the lie out.
#[test]
fn a_lie_is_found_out_by_going_there() {
    let mut listener = somebody();
    let liar = uuid::Uuid::new_v4();

    let nowhere = Position::new(40, 40);
    listener.exploration_knowledge.take_their_word_for_it(
        nowhere,
        ResourceType::Food,
        liar,
        0,
        Some(20),
        0,
    );

    // Nothing of the sort is there, and the agent is standing looking at it
    let really_here = std::collections::BTreeSet::new();
    let found_out = listener
        .exploration_knowledge
        .hearsay_in_view(nowhere, 3, &really_here);

    assert_eq!(found_out.len(), 1, "there should be exactly one thing to find out");
    assert_eq!(found_out[0].1.who, liar, "and it should be laid at his door");
}

/// What an agent saw for itself is not laid at anybody's door.
#[test]
fn what_you_saw_yourself_is_nobodys_fault() {
    let mut agent = somebody();
    let here = Position::new(40, 40);

    // Seen, not heard
    agent
        .exploration_knowledge
        .discover_resource(here, ResourceType::Food, 0);

    let really_here = std::collections::BTreeSet::new();
    assert!(
        agent
            .exploration_knowledge
            .hearsay_in_view(here, 3, &really_here)
            .is_empty(),
        "a patch that has gone since you saw it is not a lie somebody told you"
    );
}

/// An agent passes on what it saw and not what it was told, so the man who
/// invented a place is the man blamed for it.
#[test]
fn nobody_passes_on_hearsay_as_though_they_had_seen_it() {
    let mut middleman = somebody();
    let liar = uuid::Uuid::new_v4();

    let seen = Position::new(10, 10);
    let heard = Position::new(40, 40);

    middleman
        .exploration_knowledge
        .discover_resource(seen, ResourceType::Wood, 0);
    middleman
        .exploration_knowledge
        .take_their_word_for_it(heard, ResourceType::Food, liar, 0, None, 0);

    let would_pass_on = middleman.exploration_knowledge.seen_for_myself();

    assert!(
        would_pass_on.iter().any(|(where_it_is, _)| *where_it_is == seen),
        "he tells people what he has been to and looked at"
    );
    assert!(
        !would_pass_on.iter().any(|(where_it_is, _)| *where_it_is == heard),
        "and not what somebody told him, or the man who invented it is never \
         the man blamed for it"
    );
}

/// Lies get told and found out in a running settlement.
#[test]
fn lies_are_told_and_found_out_in_a_settlement() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..25 {
        population.spawn_agent(AgentConfig::default());
    }

    // Somebody in this settlement is a liar, which a random draw of traits
    // does not guarantee
    for index in 0..5 {
        population.agents[index].traits.add_trait(Trait::Dishonest);
    }

    let mut simulation = crate::analytics::Simulation::new(world, population);
    for _ in 0..4000 {
        simulation.tick();
    }

    let mut hearsay = 0usize;
    let mut caught = 0usize;
    for agent in simulation
        .population
        .agents
        .iter()
        .filter(|a| a.state.is_alive)
    {
        hearsay += agent.exploration_knowledge.who_told_me.len();
        caught += agent
            .knowledge
            .trust_ratings
            .values()
            .filter(|record| record.wrong_count > 0)
            .count();
    }

    assert!(
        hearsay > 0,
        "agents should be telling each other where things are"
    );
    assert!(
        caught > 0,
        "and somebody should have gone to a place he was told about and found \
         nothing: {hearsay} places taken on somebody's word, {caught} people \
         found out"
    );
}

/// A settlement where nobody is believed still feeds itself, because looking
/// is what finds dinner.
#[test]
fn a_settlement_of_the_suspicious_still_feeds_itself() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..25 {
        population.spawn_agent(AgentConfig::default());
    }
    for agent in population.agents.iter_mut() {
        agent.traits.add_trait(Trait::Paranoid);
    }

    let mut simulation = crate::analytics::Simulation::new(world, population);
    for _ in 0..3000 {
        simulation.tick();
    }

    let alive = simulation
        .population
        .agents
        .iter()
        .filter(|a| a.state.is_alive)
        .count();
    assert!(
        alive > 0,
        "sight and smell find food without anybody having to be believed"
    );
}

/// Skill is not what decides whether you are believed.
#[test]
fn being_good_at_something_does_not_make_you_believed() {
    let mut listener = somebody();
    let mut expert = somebody();
    expert.skills.get_skill_mut(SkillType::Herbalism).level = 10;

    let before = listener.how_far_i_trust(expert.id, &expert.traits);
    listener
        .relationships
        .get_or_create_relationship(expert.id, 0)
        .weaken(1.2);

    assert!(
        listener.how_far_i_trust(expert.id, &expert.traits) < before,
        "what he is to you outweighs what he is good at"
    );
}
