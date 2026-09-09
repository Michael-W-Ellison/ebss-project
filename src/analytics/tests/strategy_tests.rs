// src/analytics/tests/strategy_tests.rs
//! Layer 3: the ways of answering a drive, named and ranked.
//!
//! See `SATISFACTION.md`. What is asserted here is the first step: the ways
//! exist, they are told apart, they are learnable, and the written order still
//! decides for a body that has learned nothing - which is what makes this step
//! behaviour-neutral and so measurable on its own.

use crate::agents::patterns::{Element, Patterns};
use crate::agents::{AgentConfig, Population};
use crate::analytics::wanting::strategy::Strategy;
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();
    simulation
}

/// Drinking what you carry and drinking what is in front of you are two ways,
/// not one.
///
/// They come out as the same verb, which is exactly why the layer is needed: a
/// waterskin runs out and has to be refilled, a river does not and cannot be
/// carried away from. `Did("gather")` cannot tell them apart and never could.
#[test]
fn the_skin_and_the_river_are_two_ways_of_the_same_verb() {
    let ways = Strategy::all_for(DriveType::Thirst);

    assert!(ways.contains(&Strategy::DrinkWhatIsCarried));
    assert!(ways.contains(&Strategy::DrinkFromWhatIsHere));
    assert_ne!(
        Strategy::DrinkWhatIsCarried.called(),
        Strategy::DrinkFromWhatIsHere.called(),
        "two bets that produce one verb still have to be told apart"
    );
}

/// A way is written down as an element like anything else, so the arithmetic
/// that already ranks verbs and places ranks these without a second mechanism.
#[test]
fn a_way_is_an_element_and_survives_being_written_down() {
    let by = Strategy::WalkToWaterHeKnows.as_element();
    assert_eq!(by, Element::By("walk-to-water".to_string()));

    let written: String = by.clone().into();
    assert_eq!(written, "by:walk-to-water");
    assert_eq!(Element::try_from(written).unwrap(), by);
}

/// A drive with no ways yet falls through to the arm it always had.
///
/// This is how the layer goes in one drive at a time instead of in one
/// unmeasurable jump.
#[test]
fn a_drive_with_no_ways_yet_answers_as_it_always_did() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    assert!(Strategy::all_for(DriveType::Hunger).is_empty());
    assert!(
        simulation
            .the_way_to_answer(DriveType::Hunger, agent, agent.state.position)
            .is_none(),
        "no ways means no answer from this layer, and the old ladder runs"
    );
}

/// A way whose preconditions are unmet is not a candidate.
///
/// An empty pack cannot be drunk from, whatever it has been worth in the past.
/// Asserted against the precondition check itself rather than against a world
/// arranged to have no water in it: the default test map has a river on it, so
/// arranging the *absence* of a thing is the harder and more brittle half.
#[test]
fn a_way_that_cannot_be_taken_is_not_offered() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    assert_eq!(agent.inventory.available_water(), 0.0, "the pack is empty");
    assert!(
        simulation
            .can_this_way_be_taken(
                Strategy::DrinkWhatIsCarried,
                agent,
                agent.state.position
            )
            .is_none(),
        "an empty pack is not something to drink out of"
    );
}

/// And a way whose preconditions are met comes back with what to do.
#[test]
fn a_way_that_can_be_taken_says_what_it_comes_to() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    // Whatever else is true of this map, one of the three ways can be taken -
    // the layer is not allowed to leave a thirsty man with nothing at all
    // while `water_action` would have found him something.
    let any = Strategy::all_for(DriveType::Thirst).iter().any(|way| {
        simulation
            .can_this_way_be_taken(*way, agent, agent.state.position)
            .is_some()
    });
    assert!(any, "some way of answering thirst is open on an ordinary map");

    let chosen = simulation
        .the_way_to_answer(DriveType::Thirst, agent, agent.state.position)
        .expect("and so the layer names one");
    assert!(matches!(
        chosen.doing,
        Action::Gather { .. } | Action::Move { .. }
    ));
}

/// What a body has learned about a way decides between them, and the written
/// order decides when it has learned nothing.
///
/// The ranking is asserted on the trails rather than through a world arranged
/// to make all three ways open at once, which is the brittle part.
#[test]
fn what_worked_before_outranks_the_order_somebody_typed() {
    let mut patterns = Patterns::default();
    let walking = Strategy::WalkToWaterHeKnows.as_element();

    for round in 0..8u32 {
        patterns.it_worked(
            DriveType::Thirst,
            std::slice::from_ref(&walking),
            0.9,
            round,
        );
    }

    let worth_of = |way: Strategy| {
        patterns
            .trail(DriveType::Thirst, &way.as_element())
            .map(|trail| trail.worth())
            .unwrap_or(0.0)
    };

    assert!(
        worth_of(Strategy::WalkToWaterHeKnows) > worth_of(Strategy::DrinkWhatIsCarried),
        "a way that has paid is worth more than one that has not"
    );
    assert_eq!(
        worth_of(Strategy::DrinkWhatIsCarried),
        0.0,
        "and a way nobody has tried is worth nothing, so the written order \
         still decides between the untried ones - which is what makes this \
         step change no behaviour at all"
    );
}
