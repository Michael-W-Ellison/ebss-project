// src/analytics/tests/chase_tests.rs
//! A hungry hunter follows somebody who runs from it (#437).
//!
//! Until #437 nothing made a beast go after a person: hunters struck at
//! whoever stood within three paces, and a run of nineteen took anybody out
//! of it. A lion now runs down somebody who has just run from it, once in a
//! half hour, hungry or fed, if they are within its reach, and strikes one
//! time in ten when it gets there.

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
fn a_fed_lion_runs_down_somebody_who_ran_from_it_too() {
    let mut simulation = ran_from_a_lion(0.0);

    simulation.predators_try_their_luck_at(0);

    let him = simulation.population.agents[0].state.position;
    assert!(paces_between(where_the_lion_is(&simulation), him) <= 1, "running sets it off, fed or not");
    assert_eq!(simulation.world.people_run_down, 1);
}

#[test]
fn a_fed_lion_that_has_run_somebody_down_strikes_about_one_time_in_ten() {
    let mut struck = 0;
    let tries = 2000;
    for _ in 0..tries {
        let mut simulation = ran_from_a_lion(0.0);
        simulation.population.agents[0].state.health = 100.0;
        simulation.predators_try_their_luck_at(0);
        if simulation.population.agents[0].state.health < 100.0 {
            struck += 1;
        }
    }
    let share = struck as f32 / tries as f32;
    assert!(
        (0.07..=0.13).contains(&share),
        "struck {struck} times in {tries}, against {}",
        Simulation::A_CAUGHT_RUNNER_IS_STRUCK
    );
}

#[test]
fn a_fed_lion_leaves_somebody_standing_near_it_who_did_not_run() {
    let mut simulation = ran_from_a_lion(0.0);
    simulation.population.agents[0].state.ran_from = None;
    simulation.population.agents[0].state.position = (30, 33, 0);
    simulation.population.agents[0].state.health = 100.0;

    for _ in 0..200 {
        simulation.predators_try_their_luck_at(0);
    }

    assert_eq!(simulation.population.agents[0].state.health, 100.0, "a fed lion does not come at anybody standing still");
    assert_eq!(where_the_lion_is(&simulation), (30, 31));
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

/// A grown man a lion has just run down, frightened of it, with the lion a
/// pace off and his legs and health whole.
fn run_down_by_a_lion() -> Simulation {
    let mut simulation = ran_from_a_lion(0.0);
    simulation.predators_try_their_luck_at(0);
    let agent = &mut simulation.population.agents[0];
    agent.state.health = 100.0;
    agent.state.energy = 100.0;
    agent.traits = crate::core::TraitSet::new();
    let kind = simulation.world.animals.get_species("lion").expect("lions exist").name.clone();
    simulation.population.agents[0]
        .emotions
        .set_fear(crate::agents::emotions::EmotionSource::Creature(kind), 0.9);
    simulation
}

#[test]
fn somebody_a_lion_has_run_down_does_not_run_from_it_again() {
    let simulation = run_down_by_a_lion();
    assert_eq!(simulation.world.people_run_down, 1, "the lion should have run him down");

    let him = &simulation.population.agents[0];
    let (branch, answer) = simulation.what_this_threat_comes_to(him, him.state.position);

    assert_eq!(branch, "run down, covers up");
    assert!(matches!(answer, Some(crate::environment::Action::Freeze)), "{answer:?}");
}

#[test]
fn fear_alone_does_not_send_somebody_run_down_off_again() {
    let mut simulation = run_down_by_a_lion();
    // Somebody who could not face it, whom fear on its own sends running
    simulation.population.agents[0].state.life_stage = LifeStage::Child;
    let him = &simulation.population.agents[0];

    let answer = simulation.what_to_run_from(him, him.state.position);

    assert!(matches!(answer, Some(crate::environment::Action::Freeze)), "{answer:?}");
}

#[test]
fn somebody_run_down_a_day_ago_runs_as_before() {
    let mut simulation = run_down_by_a_lion();
    simulation.current_turn = NOW + crate::environment::seasons::ONCE_A_DAY;
    let him = &simulation.population.agents[0];

    let (branch, answer) = simulation.what_this_threat_comes_to(him, him.state.position);

    assert_ne!(branch, "run down, covers up");
    assert!(matches!(answer, Some(crate::environment::Action::FleeFrom { .. })), "{branch}: {answer:?}");
}

#[test]
fn somebody_run_down_by_a_polar_bear_still_runs_from_a_lion() {
    let mut simulation = run_down_by_a_lion();
    simulation.population.agents[0].state.chased_by = Some("polar_bear".to_string());
    let him = &simulation.population.agents[0];

    let (branch, answer) = simulation.what_this_threat_comes_to(him, him.state.position);

    assert_ne!(branch, "run down, covers up");
    assert!(matches!(answer, Some(crate::environment::Action::FleeFrom { .. })), "{branch}: {answer:?}");
}

/// Lions go about in twos. Run down by one, he does not run from the other
/// when it comes up beside him: it would run him down the same (#437).
#[test]
fn somebody_run_down_by_one_lion_does_not_run_from_the_other() {
    let mut simulation = run_down_by_a_lion();
    let at = simulation.population.agents[0].state.position;
    // The one that ran him down has gone off, and its mate is beside him
    simulation.world.animals.get_all_mut()[0].position = (at.0 + 60, at.1);
    simulation
        .world
        .spawn_animal("lion".to_string(), (at.0, at.1 - 1))
        .expect("a second lion should spawn");
    let him = &simulation.population.agents[0];

    let (branch, answer) = simulation.what_this_threat_comes_to(him, him.state.position);

    assert_eq!(branch, "run down, covers up");
    assert!(matches!(answer, Some(crate::environment::Action::Freeze)), "{answer:?}");
}

/// The whole of it, minute by minute, with the lion keeping two paces off
/// him: somebody who covers up is not run down again, and a fed lion
/// beside him leaves him be. Without the cover-up he runs every minute and
/// is run down again every half hour.
#[test]
fn a_fed_lion_runs_somebody_down_once_and_no_more() {
    let mut simulation = run_down_by_a_lion();
    let health = simulation.population.agents[0].state.health;
    for minute in 1..=(3 * crate::environment::seasons::MINUTES_PER_TURN) {
        simulation.current_turn = NOW + minute;
        let at = simulation.population.agents[0].state.position;
        simulation.world.animals.get_all_mut()[0].position = (at.0, at.1 - 2);
        let him = &simulation.population.agents[0];
        if let (_, Some(crate::environment::Action::FleeFrom { away_from })) =
            simulation.what_this_threat_comes_to(him, him.state.position)
        {
            simulation.fleeing_from(&away_from, 0);
        }
        simulation.predators_try_their_luck_at(0);
    }

    assert_eq!(simulation.world.people_run_down, 1);
    assert_eq!(simulation.population.agents[0].state.health, health, "a fed lion strikes nobody standing");
}
