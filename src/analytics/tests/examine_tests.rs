// src/analytics/tests/examine_tests.rs
//! Tests for looking closely at a thing you are already holding.
//!
//! The third road into the chain, beside doing a thing twice to see it happen
//! again and putting the wrong thing where a part goes. It is the cheapest of
//! the three — a turn and no materials — which is exactly why it has to pay
//! off least often: a generous version would collapse the whole chain into an
//! afternoon spent turning things over.
//!
//! And the other two sensory verbs are the other kind of verb entirely.
//! Nobody decides to smell what is rotting in the next field, or to overhear
//! what somebody says in earshot. Those happen, and the matrix says so.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::verbs;
use crate::environment::{making, Action};
use crate::world::{World, WorldConfig};

fn a_person() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);

    let everything: Vec<(String, u32)> = simulation.population.agents[0]
        .inventory
        .get_all_items()
        .values()
        .map(|item| (item.item_id.clone(), item.quantity))
        .collect();

    for (what, how_many) in everything {
        for _ in 0..how_many {
            simulation.population.agents[0]
                .inventory
                .remove_item(&what, 1);
        }
    }

    simulation
}

fn give(simulation: &mut Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            what.to_string(),
            how_many,
            1.0,
        ));
}

// --------------------------------------------------------------------------
// The sensory family in the matrix
// --------------------------------------------------------------------------

/// One of the three is a decision and two of them are not.
#[test]
fn one_sensory_verb_is_chosen_and_two_happen_to_you() {
    let examining = verbs::what_that_verb_is("examine").expect("in the matrix");
    let smelling = verbs::what_that_verb_is("smell").expect("in the matrix");
    let listening = verbs::what_that_verb_is("listen").expect("in the matrix");

    assert!(examining.is_chosen(), "a man decides to look at a thing");
    assert!(
        !smelling.is_chosen() && smelling.is_live(),
        "nobody decides to smell what is rotting; it reaches them"
    );
    assert!(
        !listening.is_chosen() && listening.is_live(),
        "and nobody decides to overhear"
    );

    for one in [smelling, listening] {
        assert!(
            one.happens_when.is_some(),
            "{} should say what occasions it",
            one.called
        );
    }
}

/// All three change what somebody knows and nothing else.
#[test]
fn the_sensory_verbs_change_what_is_known() {
    for one in verbs::EVERY_VERB
        .iter()
        .filter(|verb| verb.family == verbs::Family::Sensory)
    {
        assert_eq!(
            one.changes,
            &[verbs::Changes::WhatIsKnown],
            "{} should change what is known and leave the world alone",
            one.called
        );
    }
}

// --------------------------------------------------------------------------
// Looking at a thing
// --------------------------------------------------------------------------

/// A thing that goes into nothing anybody has left to find out tells you
/// nothing.
#[test]
fn a_familiar_thing_tells_you_nothing() {
    let mut simulation = a_person();
    give(&mut simulation, "flax", 4);

    let result = simulation.execute_action(
        &Action::Examine {
            what: "flax".to_string(),
        },
        0,
    );

    assert!(
        !result.success,
        "everybody knows what flax is for: {:?}",
        result.message
    );
}

/// And a thing you are not holding cannot be looked at.
#[test]
fn you_cannot_look_at_what_you_are_not_holding() {
    let mut simulation = a_person();

    let result = simulation.execute_action(
        &Action::Examine {
            what: "iron".to_string(),
        },
        0,
    );
    assert!(!result.success, "it has to be in your hand");
}

/// A bright stone is a thing with something in it, and looking at it long
/// enough tells you.
#[test]
fn looking_long_enough_at_a_strange_thing_tells_you_what_it_is_for() {
    // What iron goes into that nobody arrives knowing
    let waiting = making::everything_to_find_out()
        .find(|step| step.needs.iter().any(|(needs, _)| *needs == "iron"))
        .expect("a bright stone is a step towards something");

    let mut found_out = 0;

    for _ in 0..400 {
        let mut simulation = a_person();
        give(&mut simulation, "iron", 4);

        simulation.execute_action(
            &Action::Examine {
                what: "iron".to_string(),
            },
            0,
        );

        if simulation.population.agents[0]
            .what_i_found_out()
            .contains(waiting.makes)
        {
            found_out += 1;
        }
    }

    assert!(
        found_out > 0,
        "four hundred good long looks at a bright stone should tell somebody something"
    );
    assert!(
        found_out < 200,
        "and it should not be a thing that works half the time: {found_out} of 400"
    );
}

/// Having seen it, he can do it on purpose.
#[test]
fn what_looking_tells_you_is_a_thing_you_can_then_do() {
    let waiting = making::everything_to_find_out()
        .find(|step| step.needs.iter().any(|(needs, _)| *needs == "iron"))
        .expect("a bright stone is a step towards something");

    let mut simulation = a_person();
    give(&mut simulation, "iron", 4);

    assert!(
        !simulation.population.agents[0].knows_how_to(waiting),
        "nobody arrives knowing"
    );

    simulation.population.agents[0].found_out_how_to(waiting.makes);

    assert!(
        simulation.population.agents[0].knows_how_to(waiting),
        "and having seen what a bright stone is for, he can do it"
    );
}

// --------------------------------------------------------------------------
// Choosing to look
// --------------------------------------------------------------------------

/// An agent turns over the thing in its pack that might be for something.
#[test]
fn a_strange_thing_in_the_pack_is_what_gets_looked_at() {
    let mut simulation = a_person();

    // Nothing but things everybody understands. Not a stick: everybody knows
    // a fire wants wood and nobody knows what shavings are for, so a stick is
    // a question until somebody has scraped one.
    give(&mut simulation, "flax", 4);
    give(&mut simulation, "stone", 4);

    assert!(
        simulation.population.agents[0]
            .what_i_would_look_at()
            .is_none(),
        "a handful of flax and a couple of cores raise no questions"
    );

    give(&mut simulation, "iron", 2);

    // Whether he gets round to it on a given turn is a roll, so this asks
    // whether it is a thing he would do at all
    let would = (0..60)
        .filter_map(|_| simulation.population.agents[0].what_i_would_look_at())
        .any(|what| what == "iron");

    assert!(would, "a bright stone in the pack is a question");
}

/// And stops once he knows the answer.
#[test]
fn nobody_keeps_looking_at_a_thing_they_have_worked_out() {
    let mut simulation = a_person();
    give(&mut simulation, "iron", 2);

    // Everything a bright stone leads to, already known
    let answers: Vec<&'static str> = making::everything_to_find_out()
        .filter(|step| step.needs.iter().any(|(needs, _)| *needs == "iron"))
        .map(|step| step.makes)
        .collect();

    for answer in &answers {
        simulation.population.agents[0].found_out_how_to(answer);
    }

    let would = (0..60)
        .filter_map(|_| simulation.population.agents[0].what_i_would_look_at())
        .any(|what| what == "iron");

    assert!(
        !would,
        "a man who knows what a bright stone is for does not stand looking at one"
    );
}
