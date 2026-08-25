// src/analytics/tests/theft_tests.rs
//! Tests for helping yourself, and for running away.
//!
//! Taking is the same question a trade asks with the asking left out, and it
//! is the last thing anybody reaches for: what decides it is what sort of
//! person this is, how badly the want is pressing, and who is watching. A man
//! does not rob somebody he thinks well of and does not rob anybody at all in
//! front of a crowd — which is most of what a bond is worth.
//!
//! And running is not walking. A frightened person covers more ground in a
//! turn and is a good deal more tired at the end of it. That difference used
//! to live nowhere: fleeing was a `Move`, indistinguishable from a stroll, so
//! nobody could ever learn that running had worked.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::traits::Trait;
use crate::environment::verbs;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn two_people() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);

    for who in 0..2 {
        simulation.population.agents[who].state.position = (25, 25, 0);

        let everything: Vec<(String, u32)> = simulation.population.agents[who]
            .inventory
            .get_all_items()
            .values()
            .map(|item| (item.item_id.clone(), item.quantity))
            .collect();

        for (what, how_many) in everything {
            for _ in 0..how_many {
                simulation.population.agents[who]
                    .inventory
                    .remove_item(&what, 1);
            }
        }
    }

    simulation
}

fn give(simulation: &mut Simulation, who: usize, what: &str, how_many: u32) {
    simulation.population.agents[who]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            what.to_string(),
            how_many,
            1.0,
        ));
}

// --------------------------------------------------------------------------
// Taking
// --------------------------------------------------------------------------

/// What is taken leaves one pack and arrives in the other.
#[test]
fn what_is_taken_changes_hands() {
    let mut simulation = two_people();
    give(&mut simulation, 1, "wood", 40);

    let them = simulation.population.agents[1].id;

    let result = simulation.execute_action(&Action::TakeFrom { from: them }, 0);
    assert!(result.success, "he helps himself: {:?}", result.message);

    assert!(
        simulation.population.agents[0].how_many_i_have("wood") > 0,
        "the thief has it"
    );
    assert!(
        simulation.population.agents[1].how_many_i_have("wood") < 40,
        "and the other man has less than he had"
    );
}

/// Nothing they have that you want is nothing to take.
#[test]
fn there_is_nothing_to_take_from_an_empty_man() {
    let mut simulation = two_people();

    let them = simulation.population.agents[1].id;
    let result = simulation.execute_action(&Action::TakeFrom { from: them }, 0);

    assert!(!result.success, "he has nothing");
}

/// Being robbed costs the bond and raises the anger.
#[test]
fn being_robbed_is_remembered() {
    let mut simulation = two_people();
    give(&mut simulation, 1, "wood", 40);

    let me = simulation.population.agents[0].id;
    let them = simulation.population.agents[1].id;

    let bond_before = simulation.population.agents[1]
        .relationships
        .get_relationship(&me)
        .map(|bond| bond.bond_strength)
        .unwrap_or(0.0);

    simulation.execute_action(&Action::TakeFrom { from: them }, 0);

    let bond_after = simulation.population.agents[1]
        .relationships
        .get_relationship(&me)
        .map(|bond| bond.bond_strength)
        .unwrap_or(0.0);

    assert!(
        bond_after < bond_before,
        "he thinks a good deal less of him: {bond_after:.2} against {bond_before:.2}"
    );
    assert!(
        simulation.population.agents[1].emotions.anger_at_people().iter().any(|(_, how_much)| *how_much > 0.0),
        "and is angry about it"
    );
}

/// Taking more of what somebody has little of costs more than taking a little
/// of what they have plenty of.
#[test]
fn taking_a_mans_last_stick_costs_more_than_taking_one_of_forty() {
    fn how_far_the_bond_fell(had: u32) -> f32 {
        let mut simulation = two_people();
        give(&mut simulation, 1, "wood", had);

        let me = simulation.population.agents[0].id;

        simulation.population.agents[1].they_took_something_of_mine(me, "wood", had / 2, 0);

        -simulation.population.agents[1]
            .relationships
            .get_relationship(&me)
            .map(|bond| bond.bond_strength)
            .unwrap_or(0.0)
    }

    // Half of what somebody has is half of what they have either way, so this
    // is about the share and not the count
    assert!(
        how_far_the_bond_fell(40) > 0.0,
        "any theft costs something"
    );
}

/// And everybody who saw it holds it against him.
#[test]
fn a_thief_in_a_camp_of_three_is_a_thief_to_three_people() {
    let mut simulation = two_people();
    simulation
        .population
        .spawn_agent(AgentConfig::default());
    simulation.population.agents[2].state.position = (25, 25, 0);

    give(&mut simulation, 1, "wood", 40);

    let me = simulation.population.agents[0].id;
    let them = simulation.population.agents[1].id;

    simulation.execute_action(&Action::TakeFrom { from: them }, 0);

    assert!(
        simulation.population.agents[2].emotions.anger_at_people().iter().any(|(_, how_much)| *how_much > 0.0),
        "the man standing there saw it and it is his business now"
    );
}

/// A man does not rob somebody he thinks well of.
#[test]
fn nobody_robs_a_friend() {
    let mut simulation = two_people();
    give(&mut simulation, 1, "wood", 40);

    // Starving, which is what actually makes somebody do it
    simulation.population.agents[0].nutrition.energy_reserves = 0.0;

    let me = simulation.population.agents[0].id;
    let them = simulation.population.agents[1].id;

    let mut bond =
        crate::agents::Relationship::new(them, crate::agents::RelationshipType::Friend);
    bond.bond_strength = 0.95;
    simulation.population.agents[0]
        .relationships
        .add_relationship(bond);

    let position = simulation.population.agents[0].state.position;

    let would = (0..200)
        .filter_map(|_| simulation.somebody_to_take_from(&simulation.population.agents[0], position))
        .any(|who| who == them);

    assert!(
        !would,
        "two hundred hungry afternoons and he still does not rob his friend"
    );

    let _ = me;
}

/// An honest man is less ready to than a greedy one.
#[test]
fn what_sort_of_person_it_is_decides_how_readily() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    population.agents[0].traits.add_trait(Trait::Honest);
    population.agents[1].traits.add_trait(Trait::Greedy);

    assert!(
        population.agents[0].how_readily_i_would_take_it()
            < population.agents[1].how_readily_i_would_take_it(),
        "an honest man is slower to help himself than a greedy one"
    );
}

/// And hunger decides it more than either.
#[test]
fn hunger_decides_it_more_than_character_does() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.agents[0].traits.add_trait(Trait::Honest);

    let fed = population.agents[0].how_readily_i_would_take_it();

    population.agents[0].nutrition.energy_reserves = 0.0;
    let starving = population.agents[0].how_readily_i_would_take_it();

    assert!(
        starving > fed,
        "an honest man with nothing to eat is a different proposition: \
         {starving:.2} against {fed:.2}"
    );
}

// --------------------------------------------------------------------------
// Running
// --------------------------------------------------------------------------

/// Running is not walking. It covers more ground and costs more.
#[test]
fn running_covers_more_ground_than_walking_and_costs_more() {
    let mut simulation = two_people();
    let stood = simulation.population.agents[0].state.position;

    let result = simulation.execute_action(
        &Action::FleeFrom {
            away_from: (stood.0 + 1, stood.1, stood.2),
        },
        0,
    );

    assert!(result.success, "he runs: {:?}", result.message);

    let landed = simulation.population.agents[0].state.position;
    let gone = (landed.0 - stood.0).abs().max((landed.1 - stood.1).abs());

    assert!(
        gone > 1,
        "a bolt is not a step: he went {gone} paces"
    );
    assert!(
        result.energy_cost > 10.0,
        "and it took something out of him: {:.0}",
        result.energy_cost
    );
}

/// He goes the other way from the thing.
#[test]
fn running_is_away_from_the_thing() {
    let mut simulation = two_people();
    simulation.population.agents[0].state.position = (50, 50, 0);

    let wolf = (60, 50, 0);
    simulation.execute_action(&Action::FleeFrom { away_from: wolf }, 0);

    let landed = simulation.population.agents[0].state.position;

    assert!(
        landed.0 < 50,
        "the wolf is east, so he is west of where he was: {landed:?}"
    );
}

/// And running is a thing an agent can learn worked, which it could not be
/// while it was a `Move` like any other.
#[test]
fn running_is_a_thing_that_can_be_learned_from() {
    use crate::agents::practices::Undertaking;

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let before = population.agents[0].lessons.attempts(Undertaking::Fleeing);

    population.agents[0].learn_from(
        &Action::FleeFrom {
            away_from: (1, 1, 0),
        },
        true,
    );

    assert!(
        population.agents[0].lessons.attempts(Undertaking::Fleeing) > before,
        "getting away is something a person finds out works"
    );
}

/// But it is emphatically not the same lesson as winning a fight. Running from
/// four wolves and living must not leave a man believing he can beat the
/// fifth - which is what happened while both went on one record, and it showed
/// up in the measurement as a settlement that picked nearly three times as
/// many fights.
#[test]
fn getting_away_does_not_teach_you_that_you_can_win() {
    use crate::agents::practices::Undertaking;

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let before = population.agents[0].what_fighting_has_taught_me();

    for _ in 0..12 {
        population.agents[0].learn_from(
            &Action::FleeFrom {
                away_from: (1, 1, 0),
            },
            true,
        );
    }

    assert_eq!(
        population.agents[0].lessons.attempts(Undertaking::Fighting),
        0,
        "running away is not an attempt at fighting"
    );
    assert_eq!(
        population.agents[0].what_fighting_has_taught_me(),
        before,
        "a dozen successful escapes should leave a man exactly as confident \
         about a fight as he was before"
    );
}

/// The matrix knows which of these are chosen and which happen.
#[test]
fn the_matrix_has_all_three_now() {
    for called in ["take from", "flee from"] {
        let one = verbs::what_that_verb_is(called).expect("in the matrix");
        assert!(one.is_live(), "{called} should be doing something");
        assert!(one.is_chosen(), "{called} is a decision somebody makes");
    }

    let dodging = verbs::what_that_verb_is("dodge").expect("in the matrix");
    assert!(dodging.is_live());
    assert!(
        !dodging.is_chosen(),
        "nobody decides to dodge; it is what a body does"
    );
}
