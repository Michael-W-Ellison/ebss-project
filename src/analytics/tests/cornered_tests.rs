// src/analytics/tests/cornered_tests.rs
//! Tests that a man with his back to the water still runs.
//!
//! Two things asked whether there was anywhere to run, and they asked it in
//! different words. The decision tried three ways out at three paces; the
//! running tried three ways out at nineteen. Between those two numbers sits a
//! shoreline: a man three paces from the water with the thing inland has
//! somewhere to go at three paces and nothing but water at nineteen, so the
//! decision said run and the running said "Nowhere to run" - and nothing
//! about the next turn was different, so it said it again, and again. One
//! measured world produced 76,644 of those refusals, three quarters of every
//! turn taken in the settlement, and by a distance the largest single refusal
//! the model has produced.
//!
//! Both now ask the same function. It tries eight ways out rather than three,
//! and each of them at every distance down to a single pace, so the narrow
//! gap counts as a gap. And where there is genuinely nowhere - a tile with
//! water on all sides - standing your ground is an answer that costs a turn,
//! not a refusal that can be repeated forever.

use crate::agents::{AgentConfig, EmotionSource, LifeStage, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Position, TerrainType, World, WorldConfig};

/// A country of exactly the shape the test wants: passable where `dry` says
/// so and open water everywhere else.
fn a_country_shaped_like(dry: impl Fn(i32, i32) -> bool) -> World {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    let (width, height) = (world.grid.width as i32, world.grid.height as i32);
    for y in 0..height {
        for x in 0..width {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain.terrain_type = if dry(x, y) {
                    TerrainType::Plains
                } else {
                    TerrainType::Water
                };
            }
        }
    }

    world
}

fn one_person(world: World, stood: (i32, i32, i32)) -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = stood;
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation.population.agents[0].state.health = 100.0;
    simulation
}

/// A spit of land one tile wide, running east and west, with the sea either
/// side of it.
fn a_spit_of_land() -> World {
    a_country_shaped_like(|x, y| y == 30 && (20..=40).contains(&x))
}

// --------------------------------------------------------------------------
// The narrow gap counts as a gap
// --------------------------------------------------------------------------

/// The measured case. Nineteen paces west is open water; ten paces west is
/// dry land; and a frightened man takes the ten.
#[test]
fn somebody_on_a_spit_of_land_runs_as_far_as_the_land_goes() {
    let mut simulation = one_person(a_spit_of_land(), (30, 30, 0));

    let ran = simulation.execute_action(&Action::FleeFrom { away_from: (34, 30, 0) }, 0);

    assert!(
        ran.success,
        "there is dry land ten paces west, and it is not nowhere: {:?}",
        ran.message
    );

    let landed = simulation.population.agents[0].state.position;

    assert_eq!(
        landed,
        (20, 30, 0),
        "it should run to the far end of the land it has, not to {landed:?}"
    );
}

/// And what the decision promised, the running can do. This is the pairing
/// that broke: the two of them disagreeing is the whole defect.
#[test]
fn the_decision_never_promises_a_run_the_running_cannot_make() {
    let mut world = a_spit_of_land();
    world
        .spawn_animal("wolf".to_string(), (34, 30))
        .expect("a wolf should spawn");

    let mut simulation = one_person(world, (30, 30, 0));

    let kind = simulation
        .world
        .animals
        .get_species("wolf")
        .expect("wolves exist")
        .name
        .clone();

    let here = simulation.population.agents[0].state.position;
    simulation.population.agents[0]
        .emotions
        .set_fear(EmotionSource::Creature(kind), 0.9);

    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("a wolf four paces off is worth answering");

    if matches!(answer, Action::FleeFrom { .. }) {
        let ran = simulation.execute_action(&answer, 0);
        assert!(
            ran.success,
            "the decision said run and the running refused: {:?}",
            ran.message
        );
    }
}

/// The two of them agree wherever you stand on the shoreline, which is the
/// property that stops the loop rather than the one case that showed it.
#[test]
fn the_two_questions_agree_all_along_the_shore() {
    let simulation = one_person(a_spit_of_land(), (30, 30, 0));
    let remembers = &simulation.population.agents[0].exploration_knowledge;

    for stood in 20..=40 {
        for threat in 20..=40 {
            if stood == threat {
                continue;
            }

            let from = (stood, 30, 0);
            let away_from = (threat, 30);

            assert_eq!(
                simulation.is_there_anywhere_to_run(remembers, from, away_from),
                simulation
                    .where_this_one_would_run(remembers, from, away_from)
                    .is_some(),
                "standing at {stood} with the thing at {threat}, the decision and the \
                 running should be answering the same question"
            );
        }
    }
}

// --------------------------------------------------------------------------
// And where there really is nowhere
// --------------------------------------------------------------------------

/// One tile of dry land with water on every side of it. There is nothing to
/// be done, and doing nothing is what happens - once, at the cost of a turn.
#[test]
fn a_man_on_a_rock_stands_his_ground_rather_than_refusing() {
    let alone = |x: i32, y: i32| (x, y) == (30, 30);
    let mut simulation = one_person(a_country_shaped_like(alone), (30, 30, 0));

    let stood = simulation.population.agents[0].state.position;
    let ran = simulation.execute_action(&Action::FleeFrom { away_from: (34, 30, 0) }, 0);

    assert!(
        ran.success,
        "standing your ground is an answer, not a refusal: {:?}",
        ran.message
    );
    assert_eq!(
        simulation.population.agents[0].state.position, stood,
        "and it does not move, because there is nowhere to move to"
    );
    assert!(
        ran.energy_cost > 0.0,
        "it costs the turn it takes, like freezing does"
    );
}

/// And the decision does not send anybody there in the first place: with
/// nowhere to run it turns and fights, or freezes, which is what the threat
/// tree is for.
#[test]
fn nowhere_to_run_is_something_the_decision_already_knows() {
    let alone = |x: i32, y: i32| (x, y) == (30, 30);
    let simulation = one_person(a_country_shaped_like(alone), (30, 30, 0));

    assert!(
        !simulation.is_there_anywhere_to_run(
            &simulation.population.agents[0].exploration_knowledge,
            (30, 30, 0),
            (34, 30),
        ),
        "water on all four sides is nowhere to run"
    );
}

/// The refusal was not one refusal, it was the same refusal every turn
/// forever. Whatever the answer is, asking again has to keep giving one.
#[test]
fn being_cornered_does_not_become_a_refusal_that_repeats() {
    let alone = |x: i32, y: i32| (x, y) == (30, 30);
    let mut simulation = one_person(a_country_shaped_like(alone), (30, 30, 0));

    for turn in 0..8 {
        let ran = simulation.execute_action(&Action::FleeFrom { away_from: (34, 30, 0) }, 0);
        assert!(
            ran.success,
            "turn {turn} of being cornered should still be an answer: {:?}",
            ran.message
        );
    }
}

// --------------------------------------------------------------------------
// Eight ways out, and which of them it takes
// --------------------------------------------------------------------------

/// Behind is in the list of ways out, but it is the worst of them: on open
/// ground a frightened man goes away from the thing, not past it.
#[test]
fn on_open_ground_it_still_runs_away_and_not_past() {
    let mut simulation = one_person(a_country_shaped_like(|_, _| true), (25, 25, 0));

    let ran = simulation.execute_action(&Action::FleeFrom { away_from: (29, 25, 0) }, 0);
    assert!(ran.success, "open ground is somewhere to run");

    let landed = simulation.population.agents[0].state.position;

    assert!(
        landed.0 < 25,
        "the thing is east, so it should be west of where it started, not at {landed:?}"
    );
    assert!(
        (landed.0 - 29).abs() > 4,
        "and further off than the four paces it started at"
    );
}

/// A wood a man was mauled in is worse than four paces nearer the wolf. Where
/// straight away is remembered badly, one of the other seven ways is taken.
#[test]
fn it_does_not_run_into_the_wood_the_pack_lives_in() {
    let mut simulation = one_person(a_country_shaped_like(|_, _| true), (25, 25, 0));

    // Due west, which is exactly where running straight away lands
    simulation.population.agents[0].exploration_knowledge.saw_danger(
        Position::new(6, 25),
        "wolves",
        1.0,
        simulation.current_tick,
    );

    let ran = simulation.execute_action(&Action::FleeFrom { away_from: (29, 25, 0) }, 0);
    assert!(ran.success, "there are seven other ways out");

    let landed = simulation.population.agents[0].state.position;

    assert!(
        (landed.0 - 6).abs() > 3 || (landed.1 - 25).abs() > 3,
        "it should not have run into the wood at (6, 25), and it ran to {landed:?}"
    );
    assert!(
        (landed.0 - 29).abs().max((landed.1 - 25).abs()) > 4,
        "and it should still have got clear of the wolf, not stayed put at {landed:?}"
    );
}
