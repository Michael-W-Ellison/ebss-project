// src/analytics/tests/danger_cadence_tests.rs
//! Tests that somebody with something on them decides minute by minute.
//!
//! "Every 30 ticks/minutes agents should have the option of making a decision.
//! This does not apply if an agent encounters a dangerous situation, as they
//! must then make decisions minute by minute to enhance their survival odds."
//!
//! Note on the fixtures: it is not enough to write a fright into somebody's
//! emotions and tick. Fear and anger are re-appraised from what is actually
//! there every turn - see ISSUES #260 - so a wolf that does not exist is
//! forgotten before anybody acts on it, which is right. A test about being in
//! danger has to put something in the world to be in danger of.

use crate::agents::{AgentConfig, LifeStage, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::MINUTES_PER_TURN;
use crate::world::{World, WorldConfig};

fn an_empty_country() -> World {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
}

fn one_person_alone() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(an_empty_country(), population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation
}

/// A man already hurt, with a wolf at his elbow.
///
/// The hurt is not decoration. A sound adult sizes up one wolf and comes out
/// angry rather than frightened - measured at 0.21 against a gate of 0.43 -
/// so he is in no danger by the model's reckoning, and he is right: he can
/// face it. It takes four wolves to anger him past the gate, and that sits
/// within a hundredth of it, which is no place to rest a test. A man at forty
/// health reads the same wolf as more than he can cope with and comes out at
/// the fear ceiling of 0.700, in every run, at every count of wolves. That is
/// the appraisal doing exactly what it is for, and it is also the case the
/// minute clock exists to serve: the one who cannot simply turn and deal
/// with the thing.
fn one_person_and_a_wolf() -> Simulation {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (30, 31))
        .expect("a wolf should spawn");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation.population.agents[0].state.health = HURT_ENOUGH_TO_BE_AFRAID;
    simulation
}

/// What a man has left when a wolf is more than he can manage.
const HURT_ENOUGH_TO_BE_AFRAID: f32 = 40.0;

/// An ordinary turn is one decision. Nobody gets the fast clock for nothing.
#[test]
fn an_untroubled_turn_is_one_decision() {
    let mut simulation = one_person_alone();

    for _ in 0..6 {
        simulation.tick();
    }

    assert_eq!(
        simulation.minutes_spent_in_danger, 0,
        "nothing was on anybody, so nobody needed the minutes"
    );
}

/// Something on somebody puts them on the minute clock.
#[test]
fn a_man_with_a_wolf_on_him_is_asked_more_than_once() {
    let mut simulation = one_person_and_a_wolf();

    for _ in 0..8 {
        simulation.tick();
    }

    assert!(
        simulation.minutes_spent_in_danger > 0,
        "eight turns beside a wolf and nobody ever had to think twice"
    );
}

/// And the danger has to be the reason, not the man merely being hurt.
///
/// The same wolf, the same hurt man, the same eight turns - but the wolf put
/// across the valley instead of at his elbow. Nothing is on him, so nothing
/// changes about his turn: the fast clock is about what is there, not about
/// the state he is in.
#[test]
fn a_wolf_across_the_country_is_not_a_danger() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (5, 5))
        .expect("a wolf should spawn");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation.population.agents[0].state.health = HURT_ENOUGH_TO_BE_AFRAID;

    for _ in 0..8 {
        simulation.tick();
    }

    assert_eq!(
        simulation.minutes_spent_in_danger, 0,
        "the wolf was a day's walk away and the man spent the half hour on it"
    );
}

/// The half hour is a ceiling: nobody gets more minutes than the turn holds.
#[test]
fn nobody_gets_more_minutes_than_the_turn_holds() {
    let mut simulation = one_person_and_a_wolf();

    let turns = 8;
    for _ in 0..turns {
        simulation.tick();
    }

    let ceiling = turns * (MINUTES_PER_TURN as u64 - 1);
    assert!(
        simulation.minutes_spent_in_danger <= ceiling,
        "{turns} turns cannot hold more than {ceiling} extra minutes, got {}",
        simulation.minutes_spent_in_danger
    );
}

/// And a settlement with nothing after it never pays for the mechanism.
#[test]
fn the_fast_clock_costs_nothing_where_there_is_no_danger() {
    let mut quiet = one_person_alone();
    for _ in 0..12 {
        quiet.tick();
    }

    assert_eq!(
        quiet.minutes_spent_in_danger, 0,
        "the minute clock is for danger and there was none"
    );
}
