// src/analytics/tests/off_the_map_tests.rs
//! Nobody is put where a body cannot stand.
//!
//! `Move: No passable route toward destination` was 63,922 refusals over
//! eight seeded world-years - **half of every refusal left in the model** -
//! and it is not a pathfinding failure. By the time it fires the direct step,
//! a breadth-first search of four thousand tiles and all four neighbours have
//! been tried. Every one of those 63,922 reported the same thing: **standing
//! off the map, with 0 ways out.** `is_passable_tile` refuses everything
//! outside the grid, so an agent one pace past the edge has no neighbour it
//! can step to, in any direction, for the rest of its life. It cannot walk to
//! food or to water. It starves where it lies.
//!
//! Two places put them there, and neither asked where the edge was: a newborn
//! was placed at its mother's position plus a random pace in each axis, and
//! `Explore` stepped in a direction without checking anything at all.

use crate::agents::{AgentConfig, Population};
use crate::environment::Action;
use crate::world::{World, WorldConfig};

/// A world and one person standing in its very first corner.
fn somebody_in_the_corner() -> crate::analytics::Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (0, 0, 0);
    simulation
}

/// Exploring off the edge of the world leaves you where you were.
#[test]
fn nobody_explores_off_the_edge_of_the_world() {
    let mut simulation = somebody_in_the_corner();

    let result = simulation.execute_action(&Action::Explore { direction: (-1, -1, 0) }, 0);

    assert!(
        result.success,
        "looking about is still a turn spent: {:?}",
        result.message
    );
    assert_eq!(
        simulation.population.agents[0].state.position,
        (0, 0, 0),
        "he walked off the map and can never walk back"
    );
}

/// And having stayed on it, he can still walk somewhere.
#[test]
fn somebody_in_the_corner_can_still_get_out_of_it() {
    let mut simulation = somebody_in_the_corner();
    simulation.execute_action(&Action::Explore { direction: (-1, -1, 0) }, 0);

    let result = simulation.execute_action(&Action::Move { target: (5, 5, 0) }, 0);

    assert!(
        result.success,
        "boxed in after one look about: {:?}",
        result.message
    );
}

/// A child is born where its mother is standing, and not a pace to the left.
///
/// A mother on the first column put one child in three at x = -1.
#[test]
fn a_child_is_born_where_its_mother_stands() {
    use crate::agents::Agent;

    let mut mother = Agent::new(AgentConfig::default());
    let mut father = Agent::new(AgentConfig::default());
    mother.state.position = (0, 0, 0);
    father.state.position = (0, 0, 0);

    for _ in 0..32 {
        let child = crate::agents::reproduction::reproduce(&mother, &father, 0);
        assert_eq!(
            child.state.position,
            (0, 0, 0),
            "a child was put off the map, where it can never take a step"
        );
    }

    // And it is the mother's ground, not the father's. `reproduce` takes the
    // pregnant one's position where there is one and the first parent's
    // otherwise, so this is the first parent standing somewhere else.
    mother.state.position = (7, 9, 0);
    let child = crate::agents::reproduction::reproduce(&mother, &father, 0);
    assert_eq!(child.state.position, (7, 9, 0));
}

/// A target one pace past the edge does not walk anybody off it.
///
/// This is the one that was actually happening. The breadth-first search
/// exempts the goal tile from the passability check, because a goal may be a
/// barn door or a berry bush rather than open ground - and nothing under that
/// exemption asked whether the goal was on the map at all. A decision naming
/// (50, 10) on a fifty-wide grid walked the agent onto it, and the next turn
/// named (51, 10), and so out into nowhere.
#[test]
fn a_target_past_the_edge_does_not_walk_anybody_off_it() {
    let mut simulation = somebody_in_the_corner();
    let wide = simulation.world.grid.width as i32;
    simulation.population.agents[0].state.position = (wide - 1, 10, 0);

    for _ in 0..8 {
        let here = simulation.population.agents[0].state.position;
        simulation.execute_action(&Action::Move { target: (here.0 + 1, here.1, here.2) }, 0);

        let (x, y, _) = simulation.population.agents[0].state.position;
        assert!(
            x >= 0 && y >= 0 && x < wide && y < simulation.world.grid.height as i32,
            "walked off the map to {:?}, where there is no ground to stand on",
            simulation.population.agents[0].state.position
        );
    }
}

/// And the search itself never offers a step outside the world.
#[test]
fn the_search_never_offers_a_step_off_the_map() {
    let simulation = somebody_in_the_corner();
    let wide = simulation.world.grid.width as i32;

    let step = simulation.next_step_toward((wide - 1, 10, 0), (wide, 10, 0));

    assert!(
        step.is_none_or(|(x, y, _)| x >= 0 && y >= 0 && x < wide),
        "the search offered {step:?}, which is off the map"
    );
}
