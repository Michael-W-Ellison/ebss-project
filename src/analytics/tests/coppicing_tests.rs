// src/analytics/tests/coppicing_tests.rs
//! Coppicing (ISSUES_FOUND #382).
//!
//! - a coppice carries up to twice what wood never cut to the stool does,
//!   and grows out over about three years uncut
//! - somebody who knows to cuts the wood near home to the stool; somebody far
//!   from home, or who does not know to, does not
//! - cutting wood near home often enough is how it is found out

use crate::agents::{Agent, AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

const HOME: (i32, i32) = (25, 25);

/// One grown person standing at a stand of wood of `standing`, with a hearth
/// `from_home` cells off, an empty pack with room, and nothing else about.
fn at_a_stand_of_wood(standing: u32, from_home: i32) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.resources.clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation
        .world
        .put_a_node_down(ResourceNode::new(ResourceType::Wood, Position::new(HOME.0, HOME.1), standing));
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(30);
    agent.state.position = (HOME.0, HOME.1, 0);
    agent.hearth = Some((HOME.0 + from_home, HOME.1));
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.max_weight = 500.0;
    simulation
}

fn cut_wood(simulation: &mut Simulation) {
    simulation.population.agents[0].inventory.get_all_items_mut().clear();
    simulation.population.agents[0].busy_until = 0;
    let result = simulation.execute_action(&Action::Gather { resource_type: "wood".to_string() }, 0);
    assert!(result.success, "{result:?}");
}

/// A coppice cut as often as it will bear carries twice what the same wood
/// does uncut, and left alone grows out over about three years.
#[test]
fn a_coppice_carries_twice_and_grows_out_in_three_years() {
    let mut stand = ResourceNode::new(ResourceType::Wood, Position::new(0, 0), 100);
    let uncut = stand.how_heavy_a_crop_it_carries(1.0);
    stand.coppiced = 1.0;
    assert_eq!(stand.how_heavy_a_crop_it_carries(1.0), uncut * 2);
    for _ in 0..365 {
        stand.a_day_since_it_was_seen_to();
    }
    assert!(stand.coppiced > 0.6, "a year uncut is still coppice: {}", stand.coppiced);
    for _ in 0..(2 * 365) {
        stand.a_day_since_it_was_seen_to();
    }
    assert!(stand.coppiced < 0.01, "three years uncut has grown out: {}", stand.coppiced);
}

/// Somebody who knows to cuts the wood near home to the stool, and the stand
/// is the better coppiced for it; far from home, or not knowing to, nobody
/// does.
#[test]
fn wood_near_home_is_cut_to_the_stool_by_somebody_who_knows_to() {
    let mut knows = at_a_stand_of_wood(100, 0);
    knows.population.agents[0].found_out_how_to("know:coppice");
    cut_wood(&mut knows);
    assert!(knows.world.resources[0].coppiced > 0.0);
    assert_eq!(knows.world.cuts_to_the_stool, 1);

    let mut far = at_a_stand_of_wood(100, 60);
    far.population.agents[0].found_out_how_to("know:coppice");
    cut_wood(&mut far);
    assert_eq!(far.world.resources[0].coppiced, 0.0, "wood sixty cells from home is not come back to");

    let mut does_not_know = at_a_stand_of_wood(100, 0);
    cut_wood(&mut does_not_know);
    assert_eq!(does_not_know.world.resources[0].coppiced, 0.0);
}

/// Cutting the wood near home often enough is how somebody sees the stumps
/// sprout, and thinks to cut to the stool.
#[test]
fn cutting_wood_near_home_is_how_it_is_found_out() {
    let mut simulation = at_a_stand_of_wood(100_000, 0);
    for _ in 0..Agent::CUTS_BEFORE_COPPICING {
        assert!(!simulation.population.agents[0].knows_the_way("coppice"));
        cut_wood(&mut simulation);
    }
    assert!(simulation.population.agents[0].knows_the_way("coppice"));
}
