// src/analytics/tests/chase_tests.rs
//! A hungry hunter follows somebody who runs from it (#437).
//!
//! Until #437 nothing made a beast go after a person: hunters struck at
//! whoever stood within three paces, and a run of nineteen took anybody out
//! of it. A lion now runs down somebody who has just run from it, once in a
//! half hour, if it is hungry and they are within its reach.

use crate::agents::{AgentConfig, LifeStage, Population};
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};

const NOW: u32 = 600;

/// A grown man who has just run nineteen paces from a lion.
fn ran_from_a_lion(how_hungry: f32) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
        .spawn_animal("lion".to_string(), (30, 31))
        .expect("a lion should spawn");
    for animal in world.animals.get_all_mut() {
        animal.hunger = animal.max_hunger * how_hungry;
    }

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.current_turn = NOW;
    let agent = &mut simulation.population.agents[0];
    agent.state.life_stage = LifeStage::Adult;
    agent.state.position = (30, 50, 0);
    agent.state.ran_from = Some((30, 31, NOW));
    simulation
}

fn where_the_lion_is(simulation: &Simulation) -> (i32, i32) {
    simulation.world.animals.get_all()[0].position
}

fn paces_between(one: (i32, i32), other: (i32, i32, i32)) -> i32 {
    (one.0 - other.0).abs().max((one.1 - other.1).abs())
}

#[test]
fn a_starving_lion_runs_down_somebody_who_ran_from_it() {
    let mut simulation = ran_from_a_lion(1.0);

    simulation.predators_try_their_luck_at(0);

    let him = simulation.population.agents[0].state.position;
    assert!(
        paces_between(where_the_lion_is(&simulation), him) <= 1,
        "the lion is at {:?} and he is at {him:?}",
        where_the_lion_is(&simulation)
    );
    assert_eq!(simulation.population.agents[0].state.chased_at, Some(NOW));
    assert_eq!(simulation.world.people_run_down, 1);
}

#[test]
fn a_fed_lion_lets_somebody_who_ran_go() {
    let mut simulation = ran_from_a_lion(0.0);

    simulation.predators_try_their_luck_at(0);

    assert_eq!(where_the_lion_is(&simulation), (30, 31), "a full lion does not give chase");
    assert_eq!(simulation.world.people_run_down, 0);
}

#[test]
fn a_lion_runs_somebody_down_once_in_a_half_hour() {
    let mut simulation = ran_from_a_lion(1.0);
    simulation.population.agents[0].state.chased_at = Some(NOW - 5);

    simulation.predators_try_their_luck_at(0);

    assert_eq!(where_the_lion_is(&simulation), (30, 31), "shaken off once, it lets them go");
}

#[test]
fn a_lion_does_not_follow_somebody_who_did_not_run_from_it() {
    let mut simulation = ran_from_a_lion(1.0);
    simulation.population.agents[0].state.ran_from = Some((80, 80, NOW));

    simulation.predators_try_their_luck_at(0);

    assert_eq!(where_the_lion_is(&simulation), (30, 31), "they ran from something else");
}

#[test]
fn a_lion_does_not_follow_a_run_long_over() {
    let mut simulation = ran_from_a_lion(1.0);
    simulation.population.agents[0].state.ran_from = Some((30, 31, NOW - 10));

    simulation.predators_try_their_luck_at(0);

    assert_eq!(where_the_lion_is(&simulation), (30, 31), "that run was ten minutes ago");
}

#[test]
fn somebody_past_a_lions_reach_has_got_away() {
    let mut simulation = ran_from_a_lion(1.0);
    let lion = simulation.world.animals.get_all()[0].clone();
    let species = simulation.world.animals.get_species("lion").unwrap().clone();
    let reach = Simulation::how_far_it_runs_somebody_down(&species, &lion);
    simulation.population.agents[0].state.position = (30, 31 + reach + 4, 0);

    simulation.predators_try_their_luck_at(0);

    assert_eq!(where_the_lion_is(&simulation), (30, 31), "out of its reach at {reach}");
}
