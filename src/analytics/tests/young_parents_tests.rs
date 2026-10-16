// src/analytics/tests/young_parents_tests.rs
//! A small child is fed by whoever it is kept with.
//!
//! Two passes look after a child under six: one puts it where its people are,
//! the other feeds it out of them. They chose its people out of two different
//! lists - anybody six or over for the keeping, sixteen or over for the
//! feeding - so a child whose parent was an adolescent was kept beside that
//! parent and fed by nobody. See ISSUES_FOUND #304.

use crate::agents::{Agent, AgentConfig, Population};
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};

/// The passes a turn runs over the small children, in the turn's order.
fn look_after_the_small_ones(simulation: &mut Simulation) {
    simulation.hand_the_small_ones_over();
    simulation.the_small_stay_with_their_people();
    simulation.feed_the_small_children();
}

/// One parent of `parent_age`, their infant, and a grown stranger far off.
fn a_parent_with_an_infant(parent_age: u32) -> Simulation {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    let parent = &mut simulation.population.agents[0];
    parent.state.now_this_many_years_old(parent_age);
    parent.state.position = (20, 20, 0);
    parent.state.physiology.hydration = 1.0;
    parent.state.physiology.reserve = parent.state.physiology.reserve_capacity;
    let parent = parent.id;

    let stranger = &mut simulation.population.agents[1];
    stranger.state.now_this_many_years_old(30);
    stranger.state.position = (60, 20, 0);

    let mut child = Agent::with_parents(AgentConfig::default(), vec![parent], simulation.current_turn);
    child.state.now_this_many_years_old(1);
    child.state.position = (20, 20, 0);
    child.state.physiology.hydration = 0.5;
    simulation.population.agents.push(child);
    simulation
}

/// A grown parent waters their infant: the case that always worked.
#[test]
fn a_grown_parent_waters_their_infant() {
    let mut simulation = a_parent_with_an_infant(30);
    look_after_the_small_ones(&mut simulation);
    assert!(simulation.population.agents[2].state.physiology.hydration > 0.5);
}

/// And so does a parent of thirteen.
///
/// The child was put at the parent's feet and then looked for among the
/// grown, found nobody within a few paces, and went without.
#[test]
fn a_young_parent_waters_their_infant() {
    let mut simulation = a_parent_with_an_infant(13);
    look_after_the_small_ones(&mut simulation);
    assert_eq!(simulation.population.agents[2].state.position, (20, 20, 0), "kept with its parent");
    assert!(
        simulation.population.agents[2].state.physiology.hydration > 0.5,
        "kept beside a parent of thirteen and given nothing to drink"
    );
    assert!(simulation.population.agents[0].state.physiology.also_feeding > 0.0, "fed out of that parent");
}

/// An orphan is kept with, and fed by, the same grown body.
///
/// With its parent dead, the keeping pass took the nearest body six or over -
/// a brother of nine standing beside it - and the feeding pass the nearest of
/// sixteen or over, forty paces off.
#[test]
fn an_orphan_is_fed_by_whoever_it_is_kept_with() {
    let mut simulation = a_parent_with_an_infant(30);
    simulation.population.agents[0].state.is_alive = false;

    let mut brother = Agent::new(AgentConfig::default());
    brother.state.now_this_many_years_old(9);
    brother.state.position = (21, 20, 0);
    simulation.population.agents.push(brother);

    look_after_the_small_ones(&mut simulation);
    assert!(
        simulation.population.agents[2].state.physiology.hydration > 0.5,
        "an orphan was kept beside a child and fed by nobody"
    );
    assert_eq!(simulation.population.agents[2].state.position, (60, 20, 0), "kept with the grown stranger");
}
