// src/analytics/tests/keeping_stands_tests.rs
//! Selective harvesting, and the larger stands it keeps (ISSUES_FOUND #380).
//!
//! - a kept stand carries up to twice what the wild one does
//! - somebody who knows to, at a stand near home, leaves a quarter of it and
//!   keeps it; somebody who does not strips it as ever
//! - nobody harvests sparingly far from home, or starving
//! - coming back to the stands near home often enough is how it is found out

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

const HOME: (i32, i32) = (25, 25);

/// One grown person standing on a berry stand of `standing`, with a hearth
/// `from_home` cells off, an empty pack with room, and nothing else about.
fn at_a_berry_stand(standing: u32, from_home: i32) -> (Simulation, usize) {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.resources.clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let stand = simulation
        .world
        .put_a_node_down(ResourceNode::new(ResourceType::Food, Position::new(HOME.0, HOME.1), standing));
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(30);
    agent.state.position = (HOME.0, HOME.1, 0);
    agent.hearth = Some((HOME.0 + from_home, HOME.1));
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.max_weight = 500.0;
    (simulation, stand)
}

fn gather(simulation: &mut Simulation) -> u32 {
    let before = simulation.world.resources[0].amount;
    simulation.population.agents[0].busy_until = 0;
    let result = simulation.execute_action(&Action::Gather { resource_type: "food".to_string() }, 0);
    assert!(result.success, "{result:?}");
    before - simulation.world.resources[0].amount
}

/// A stand kept as well as a stand can be carries twice what it did wild.
#[test]
fn a_kept_stand_carries_twice_the_wild_one() {
    let mut stand = ResourceNode::new(ResourceType::Food, Position::new(0, 0), 100);
    let wild = stand.how_heavy_a_crop_it_carries(1.0);
    stand.tended = 1.0;
    assert_eq!(stand.how_heavy_a_crop_it_carries(1.0), wild * 2);
    stand.tended = 0.5;
    assert!(stand.how_heavy_a_crop_it_carries(1.0) > wild);
}

/// Somebody who knows to take sparingly, at a stand near home, leaves a
/// quarter of what stands and the stand is the better kept; somebody who
/// does not takes what a trip takes.
#[test]
fn a_sparing_harvest_leaves_a_quarter_and_keeps_the_stand() {
    let (mut sparing, _) = at_a_berry_stand(8, 0);
    sparing.population.agents[0].found_out_how_to("know:selective_harvest");
    let taken = gather(&mut sparing);
    let left = sparing.world.resources[0].amount;
    assert!(left >= 2, "a quarter of eight is left standing, not {left}");
    assert!(taken > 0);
    assert!(sparing.world.resources[0].tended > 0.0, "and the stand is kept for it");
    assert_eq!(sparing.world.sparing_harvests, 1);

    let (mut stripping, _) = at_a_berry_stand(8, 0);
    let _ = gather(&mut stripping);
    assert_eq!(stripping.world.resources[0].tended, 0.0, "nobody kept it who does not know to");
}

/// Not far from home, where nobody is coming back to it, and not starving.
#[test]
fn nobody_harvests_sparingly_far_from_home_or_starving() {
    let (far, stand) = at_a_berry_stand(40, 60);
    let mut far = far;
    far.population.agents[0].found_out_how_to("know:selective_harvest");
    assert!(!far.would_this_one_harvest_sparingly(0, stand), "a stand sixty cells from home is stripped");

    let (mut starving, stand) = at_a_berry_stand(40, 0);
    starving.population.agents[0].found_out_how_to("know:selective_harvest");
    assert!(starving.would_this_one_harvest_sparingly(0, stand));
    starving.population.agents[0].state.physiology.four_days_without_food();
    assert!(!starving.would_this_one_harvest_sparingly(0, stand), "a starving man takes what is there");
}

/// Coming back to the stands near home often enough is how somebody notices
/// that the ones not stripped come back best.
#[test]
fn coming_back_to_the_stands_near_home_is_how_it_is_found_out() {
    let (mut simulation, _) = at_a_berry_stand(10_000, 0);
    assert!(!simulation.population.agents[0].knows_the_way("selective_harvest"));
    for _ in 0..crate::agents::Agent::RETURNS_BEFORE_SPARING {
        simulation.population.agents[0].inventory.get_all_items_mut().clear();
        let _ = gather(&mut simulation);
    }
    assert!(simulation.population.agents[0].knows_the_way("selective_harvest"));
}
