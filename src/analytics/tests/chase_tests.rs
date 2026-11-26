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

/// A grown person who has just run nineteen paces from a lion.
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

    let them = simulation.population.agents[0].state.position;
    assert_eq!(
        paces_between(where_the_lion_is(&simulation), them),
        Simulation::PREDATOR_STRIKE_RANGE,
        "the lion is at {:?} and they are at {them:?}",
        where_the_lion_is(&simulation)
    );
    assert_eq!(simulation.population.agents[0].state.chased_at, Some(NOW));
    assert_eq!(simulation.world.people_run_down, 1);
}

#[test]
fn a_fed_lion_runs_down_somebody_who_ran_from_it_too() {
    let mut simulation = ran_from_a_lion(0.0);

    simulation.predators_try_their_luck_at(0);

    let them = simulation.population.agents[0].state.position;
    assert_eq!(
        paces_between(where_the_lion_is(&simulation), them),
        Simulation::PREDATOR_STRIKE_RANGE,
        "running sets it off, fed or not"
    );
    assert_eq!(simulation.world.people_run_down, 1);
}

/// It stops a strike's length short of them, on the way it came: near
/// enough to strike, a pace past a spear. Stopping a stride behind them,
/// people killed it there - lions killed by people went from 3 to 17 in
/// seventeen world-draws, 15 of them within two days of a chase (#440).
#[test]
fn a_lion_that_runs_somebody_down_stops_a_strike_short_of_them() {
    let mut simulation = ran_from_a_lion(1.0);

    simulation.predators_try_their_luck_at(0);

    let them = simulation.population.agents[0].state.position;
    assert_eq!(where_the_lion_is(&simulation), (30, 50 - Simulation::PREDATOR_STRIKE_RANGE), "on the side it came from");
    let off = paces_between(where_the_lion_is(&simulation), them);
    assert!(off > Simulation::HUNT_REACH, "{off} paces off is within a spear's reach");
    assert!(off <= Simulation::PREDATOR_STRIKE_RANGE, "{off} paces off is past its own strike");
    assert!(
        simulation.run_down_by_it_already(&simulation.population.agents[0], &simulation.world.animals.get_all()[0]),
        "they still know it has them"
    );
}

/// Somebody who ran and is still within a strike of it is not closed on
/// any further.
#[test]
fn a_lion_already_within_a_strike_of_somebody_stays_where_it_is() {
    let mut simulation = ran_from_a_lion(0.0);
    simulation.population.agents[0].state.position = (30, 33, 0);

    simulation.predators_try_their_luck_at(0);

    assert_eq!(where_the_lion_is(&simulation), (30, 31));
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

/// A grown person a lion has just run down, frightened of it, with the lion
/// a pace off and their legs and health whole.
fn run_down_by_a_lion() -> Simulation {
    let mut simulation = ran_from_a_lion(0.0);
    simulation.predators_try_their_luck_at(0);
    // Standing over them, as it does once it has got there
    let them = simulation.population.agents[0].id;
    simulation.world.animals.get_all_mut()[0].state =
        crate::environment::fauna::AnimalState::Attacking { target_id: them };
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
    assert_eq!(simulation.world.people_run_down, 1, "the lion should have run them down");

    let them = &simulation.population.agents[0];
    let (branch, answer) = simulation.what_this_threat_comes_to(them, them.state.position);

    assert_eq!(branch, "run down, covers up");
    assert!(matches!(answer, Some(crate::environment::Action::Freeze)), "{answer:?}");
}

#[test]
fn fear_alone_does_not_send_somebody_run_down_off_again() {
    let mut simulation = run_down_by_a_lion();
    // Somebody who could not face it, whom fear on its own sends running
    simulation.population.agents[0].state.life_stage = LifeStage::Child;
    let them = &simulation.population.agents[0];

    let answer = simulation.what_to_run_from(them, them.state.position);

    assert!(matches!(answer, Some(crate::environment::Action::Freeze)), "{answer:?}");
}

/// Not for a day but for as long as a lion is near: a day on, with one
/// still beside them, they stay where they are (#437).
#[test]
fn somebody_run_down_a_day_ago_with_a_lion_still_near_stays() {
    let mut simulation = run_down_by_a_lion();
    simulation.current_turn = NOW + 2 * crate::environment::seasons::ONCE_A_DAY;
    simulation.predators_try_their_luck_at(0);
    let them = &simulation.population.agents[0];

    let (branch, answer) = simulation.what_this_threat_comes_to(them, them.state.position);

    assert_eq!(branch, "run down, covers up");
    assert!(matches!(answer, Some(crate::environment::Action::Freeze)), "{answer:?}");
}

/// Once no lion is within reach of them they have got away, and run from
/// the next one that comes up as from any.
#[test]
fn somebody_who_got_out_of_a_lions_reach_runs_from_the_next_as_before() {
    let mut simulation = run_down_by_a_lion();
    let at = simulation.population.agents[0].state.position;
    simulation.world.animals.get_all_mut()[0].position = (at.0 + 200, at.1);
    simulation.current_turn = NOW + 1;
    simulation.predators_try_their_luck_at(0);
    assert_eq!(simulation.population.agents[0].state.chased_by, None, "out of reach, the memory goes");

    simulation.world.animals.get_all_mut()[0].position = (at.0, at.1 - 1);
    let them = &simulation.population.agents[0];
    let (branch, answer) = simulation.what_this_threat_comes_to(them, them.state.position);

    assert_ne!(branch, "run down, covers up");
    assert!(matches!(answer, Some(crate::environment::Action::FleeFrom { .. })), "{branch}: {answer:?}");
}

#[test]
fn somebody_run_down_by_a_polar_bear_still_runs_from_a_lion() {
    let mut simulation = run_down_by_a_lion();
    simulation.population.agents[0].state.chased_by = Some("polar_bear".to_string());
    let them = &simulation.population.agents[0];

    let (branch, answer) = simulation.what_this_threat_comes_to(them, them.state.position);

    assert_ne!(branch, "run down, covers up");
    assert!(matches!(answer, Some(crate::environment::Action::FleeFrom { .. })), "{branch}: {answer:?}");
}

/// Lions go about in twos. Run down by one, they do not run from the other
/// when it comes up and stands over them: it would run them down the same
/// (#437).
#[test]
fn somebody_run_down_by_one_lion_does_not_run_from_the_other() {
    let mut simulation = run_down_by_a_lion();
    let at = simulation.population.agents[0].state.position;
    // The one that ran them down has gone off, and its mate is beside them
    simulation.world.animals.get_all_mut()[0].position = (at.0 + 60, at.1);
    simulation
        .world
        .spawn_animal("lion".to_string(), (at.0, at.1 - 1))
        .expect("a second lion should spawn");
    let who = simulation.population.agents[0].id;
    simulation.world.animals.get_all_mut()[1].state =
        crate::environment::fauna::AnimalState::Attacking { target_id: who };
    let them = &simulation.population.agents[0];

    let (branch, answer) = simulation.what_this_threat_comes_to(them, them.state.position);

    assert_eq!(branch, "run down, covers up");
    assert!(matches!(answer, Some(crate::environment::Action::Freeze)), "{answer:?}");
}

/// The whole of it, minute by minute, with the lion keeping two paces off
/// them: somebody who covers up is not run down again, and a fed lion
/// beside them leaves them be. Without the cover-up they run every minute
/// and are run down again every half hour.
#[test]
fn a_fed_lion_runs_somebody_down_once_and_no_more() {
    let mut simulation = run_down_by_a_lion();
    let health = simulation.population.agents[0].state.health;
    for minute in 1..=(3 * crate::environment::seasons::MINUTES_PER_TURN) {
        simulation.current_turn = NOW + minute;
        let at = simulation.population.agents[0].state.position;
        simulation.world.animals.get_all_mut()[0].position = (at.0, at.1 - 2);
        let them = &simulation.population.agents[0];
        if let (_, Some(crate::environment::Action::FleeFrom { away_from })) =
            simulation.what_this_threat_comes_to(them, them.state.position)
        {
            simulation.fleeing_from(&away_from, 0);
        }
        simulation.predators_try_their_luck_at(0);
    }

    assert_eq!(simulation.world.people_run_down, 1);
    assert_eq!(simulation.population.agents[0].state.health, health, "a fed lion strikes nobody standing");
}

/// Out of its strike but within its reach, they neither run nor cover up:
/// they go about what they were doing (#437).
#[test]
fn somebody_run_down_walks_on_past_a_lion_out_of_its_strike() {
    let mut simulation = run_down_by_a_lion();
    let at = simulation.population.agents[0].state.position;
    simulation.world.animals.get_all_mut()[0].position = (at.0, at.1 - 8);
    let them = &simulation.population.agents[0];

    let (branch, answer) = simulation.what_this_threat_comes_to(them, them.state.position);

    assert_eq!(branch, "run down, walks on");
    assert!(answer.is_none(), "{answer:?}");
}

/// The fear drive asks only about what they could not face: a child here.
#[test]
fn fear_alone_does_not_hold_somebody_run_down_still_past_a_lions_strike() {
    let mut simulation = run_down_by_a_lion();
    simulation.population.agents[0].state.life_stage = LifeStage::Child;
    let at = simulation.population.agents[0].state.position;
    simulation.world.animals.get_all_mut()[0].position = (at.0, at.1 - 8);
    let them = &simulation.population.agents[0];

    assert!(simulation.what_to_run_from(them, them.state.position).is_none());
    assert!(simulation.walks_on_past_what_ran_me_down(them, them.state.position));
}

#[test]
fn somebody_never_run_down_runs_from_a_lion_out_of_its_strike() {
    let mut simulation = run_down_by_a_lion();
    simulation.population.agents[0].state.chased_by = None;
    let at = simulation.population.agents[0].state.position;
    simulation.world.animals.get_all_mut()[0].position = (at.0, at.1 - 8);
    let them = &simulation.population.agents[0];

    let (branch, answer) = simulation.what_this_threat_comes_to(them, them.state.position);

    assert!(matches!(answer, Some(crate::environment::Action::FleeFrom { .. })), "{branch}: {answer:?}");
    assert!(!simulation.walks_on_past_what_ran_me_down(them, them.state.position));
}

fn stands_its_ground_on_them(simulation: &Simulation) -> bool {
    let them = simulation.population.agents[0].id;
    matches!(
        simulation.world.animals.get_all()[0].state,
        crate::environment::fauna::AnimalState::Attacking { target_id } if target_id == them
    )
}

/// A lion beside somebody who is not covered up stands its ground on them.
#[test]
fn a_lion_beside_somebody_stands_its_ground_on_them() {
    let mut simulation = run_down_by_a_lion();

    simulation.what_the_beasts_make_of_us();

    assert!(stands_its_ground_on_them(&simulation), "{:?}", simulation.world.animals.get_all()[0].state);
}

/// Covered up for a lion that has run them down, they are no threat to it,
/// and it does not stand over them: four fed lions stood a pace off
/// somebody covered up for five days, and the person died of thirst and
/// their wounds (#437).
#[test]
fn a_lion_does_not_stand_its_ground_on_somebody_covered_up_for_it() {
    let mut simulation = run_down_by_a_lion();
    let at = simulation.population.agents[0].state.position;
    simulation.world.animals.get_all_mut()[0].position = (at.0, at.1 - 1);

    simulation.freezing(0);
    assert_eq!(simulation.population.agents[0].state.covered_up_at, Some(NOW));
    // Its nerve run out, it looks again
    simulation.world.animals.get_all_mut()[0].state = crate::environment::fauna::AnimalState::Idle;
    simulation.what_the_beasts_make_of_us();

    assert!(!stands_its_ground_on_them(&simulation), "{:?}", simulation.world.animals.get_all()[0].state);
}

/// A lion that has let up on somebody it ran down, and lies idle beside
/// them, does not keep them covered up: they get up and walk away. Kept
/// down a pace off lions that had let up, a wounded person lay four days and
/// died of the wounds and thirst (#437).
#[test]
fn somebody_run_down_walks_away_from_a_lion_that_has_let_up() {
    let mut simulation = run_down_by_a_lion();
    let at = simulation.population.agents[0].state.position;
    simulation.world.animals.get_all_mut()[0].position = (at.0, at.1 - 1);
    simulation.world.animals.get_all_mut()[0].state = crate::environment::fauna::AnimalState::Idle;

    let them = &simulation.population.agents[0];
    let (branch, answer) = simulation.what_this_threat_comes_to(them, them.state.position);
    assert_eq!(branch, "run down, walks on");
    assert_eq!(answer, None);
    assert!(simulation.walks_on_past_what_ran_me_down(them, them.state.position) || simulation.what_to_run_from(them, them.state.position).is_none());
}

/// Over whole half hours: covered up while the lion stands over them, and
/// once it has let up they are no longer kept down beside it.
#[test]
fn somebody_run_down_is_not_kept_down_by_a_lion_lying_beside_them() {
    let mut simulation = run_down_by_a_lion();
    let at = simulation.population.agents[0].state.position;
    simulation.world.animals.get_all_mut()[0].position = (at.0, at.1 - 1);

    simulation.take_a_turn();
    simulation.take_a_turn();
    assert!(simulation.population.agents[0].state.covered_up_at.is_some(), "covered up while it stands over them");

    let mut last_covered = None;
    for _ in 0..12 {
        simulation.take_a_turn();
        let them = &simulation.population.agents[0];
        if them.state.covered_up_at == Some(simulation.current_turn) {
            last_covered = Some(simulation.current_turn);
        }
    }
    let lion = simulation.world.animals.get_all()[0].clone();
    let them = &simulation.population.agents[0];
    assert!(
        last_covered.is_none_or(|t| t + 4 * crate::environment::seasons::MINUTES_PER_TURN < simulation.current_turn),
        "still covered up at {last_covered:?} (now {}), lion {:?} at {:?}, them at {:?}",
        simulation.current_turn, lion.state, lion.position, them.state.position
    );
}
