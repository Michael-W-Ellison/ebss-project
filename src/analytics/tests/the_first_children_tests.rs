// src/analytics/tests/the_first_children_tests.rs
//! The first children a settlement bore died in their first six weeks, fed,
//! watered and full of milk.
//!
//! A child under six takes no turn and never eats for itself: whatever it gets
//! comes from a grown person's body through `feed_the_small_children`. Two
//! things about a body are kept up only by eating for yourself, and neither
//! had been taught about such a child. See ISSUES_FOUND #254.

use crate::agents::{Agent, AgentConfig, Population};
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};

/// A parent and a newborn of theirs, side by side.
fn a_parent_and_a_newborn() -> Simulation {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    let parent = simulation.population.agents[0].id;
    let at = simulation.population.agents[0].state.position;
    let mut child = Agent::with_parents(AgentConfig::default(), vec![parent], simulation.current_turn);
    child.state.position = at;
    simulation.population.agents.push(child);
    simulation
}

/// The metabolism in `Agent::take_a_turn` spares a child too young to eat for
/// itself, as `AgentState::age_turn_with_modifier` already did. It did not,
/// and a newborn's energy ran dry in three weeks.
#[test]
fn a_newborn_is_not_worn_down_by_turns_it_does_not_take() {
    let mut newborn = Agent::with_parents(AgentConfig::default(), vec![], 0);
    let before = newborn.state.energy;

    for _ in 0..200 {
        newborn.take_a_turn();
    }

    assert!(
        newborn.state.energy >= before,
        "two hundred turns of being carried took a newborn's energy from {before:.1} to {:.1}",
        newborn.state.energy
    );
}

/// And what a parent feeds a small child reaches the whole of it: the body's
/// store of energy, protein and the rest as well as the reserve hunger is
/// reckoned on. Felt energy follows the first of those, so a child fed only
/// into the second went to nought with a full belly.
#[test]
fn a_child_fed_by_a_parent_is_fed_all_through() {
    let mut simulation = a_parent_and_a_newborn();
    let child = simulation.population.agents.len() - 1;

    simulation.population.agents[child].nutrition.energy_reserves = 50.0;
    simulation.population.agents[child].nutrition.protein_stores = 50.0;
    simulation.population.agents[child].nutrition.micronutrient_level = 50.0;

    simulation.feed_the_small_children();

    let fed = &simulation.population.agents[child].nutrition;
    assert!(fed.energy_reserves > 50.0, "no energy reached the child: {:.3}", fed.energy_reserves);
    assert!(fed.protein_stores > 50.0, "no protein reached the child: {:.3}", fed.protein_stores);
    assert!(
        fed.micronutrient_level > 50.0,
        "nothing fresh reached the child: {:.3}",
        fed.micronutrient_level
    );
}

/// A turn's feeding covers a turn's metabolism at its busiest, so a child fed
/// in full holds steady.
#[test]
fn a_turn_of_being_fed_covers_a_turn_of_living() {
    let mut state = crate::world::nutrition::NutritionalState::new();
    let before = (state.energy_reserves, state.protein_stores, state.micronutrient_level);

    state.turn_metabolism(1.0);
    state.consume(&crate::world::nutrition::what_a_turn_of_being_fed_is_worth());

    assert!(state.energy_reserves >= before.0 - 1e-4);
    assert!(state.protein_stores >= before.1 - 1e-4);
    assert!(state.micronutrient_level >= before.2 - 1e-4);
}

/// A week of the worst illness there is costs a healthy body about a quarter
/// of itself, and is written down as illness.
///
/// It cost 84 a week, because the rate was written a turn at a time on a
/// calendar with a third as many turns in a week - so a wound that turned
/// killed a fed, dry man in four days. And it wrote the body's health
/// directly, so what killed him was booked to whatever had last touched him,
/// and a body at nought walked about for days.
#[test]
fn a_week_of_illness_is_a_bad_week_and_not_a_sentence() {
    use crate::environment::seasons::{PLANNING_PERIODS_PER_DAY, TICKS_BETWEEN_PLANS, TICKS_PER_DAY};

    let mut sick = Agent::new(AgentConfig::default());
    let a_week = 7 * TICKS_PER_DAY;
    sick.state.ailing = Some(crate::agents::agent::Ailment {
        from: Agent::OFF_A_WOUND_THAT_TURNED.to_string(),
        since: 0,
        until: a_week,
        severity: 1.0,
        at_its_worst: 1.0,
    });

    for turn in 0..(7 * PLANNING_PERIODS_PER_DAY) {
        sick.state.physiology.hydration = 1.0;
        sick.process_survival_turn(turn * TICKS_BETWEEN_PLANS);
    }

    assert!(sick.state.is_alive, "a week of illness killed a healthy body");
    assert!(
        (60.0..90.0).contains(&sick.state.health),
        "a week at its worst left {:.1} health, where about a quarter should go",
        sick.state.health
    );
    assert_eq!(
        sick.state.what_took_the_most(),
        Some(crate::agents::AgentState::ILLNESS),
        "and what took it should be on the record as illness"
    );
}

/// A parent feeding a small child eats for it.
///
/// Every sitting stopped at a third of a grown day and answered a hunger as
/// though it had fed one body, so parents took in the same whatever they were
/// passing on, sat at two-thirds of their reserve, and handed their children
/// three-quarters of a feed. See ISSUES_FOUND #256.
#[test]
fn a_parent_eats_for_the_child_they_feed() {
    let mut simulation = a_parent_and_a_newborn();
    simulation.population.agents[0].state.now_this_many_years_old(30);

    simulation.feed_the_small_children();

    let parent = &simulation.population.agents[0].state.physiology;
    let newborn_share = crate::agents::agent::what_a_body_this_age_eats(0);
    assert!(
        (parent.also_feeding - newborn_share).abs() < 0.1,
        "a parent with a newborn should be eating a fifth again: {:.2}",
        parent.also_feeding
    );

    let a_sitting = crate::agents::physiology::WHAT_A_SITTING_AIMS_AT;
    assert!(parent.what_a_sitting_is_for_whoever_it_feeds() > a_sitting);
    assert!(
        parent.what_this_meal_answers_here(a_sitting) < 1.0,
        "one body's supper is not all of a hunger that is eating for two"
    );

    // And once the child is gone, nobody is eaten for.
    simulation.population.agents[1].state.is_alive = false;
    simulation.feed_the_small_children();
    assert_eq!(simulation.population.agents[0].state.physiology.also_feeding, 0.0);
}
