// src/analytics/tests/clean_water_tests.rs
//! Choosing where to drink (ISSUES_FOUND #391).
//!
//! "Selection of preferred springs over surface water", stage 1 of the water
//! path.
//!
//! - somebody who knows to passes by water with fouled ground about it for
//!   clean water within reach; somebody who does not drinks the nearest
//! - with nothing clean within reach, a drink is a drink
//! - being laid up from foul water twice is how it is found out

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::world::soil::Soil;
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

const HERE: (i32, i32) = (25, 25);
const FOUL: (i32, i32) = (26, 25);
const CLEAN: (i32, i32) = (32, 25);

/// One grown person by a fouled pool, with a clean one a little way off if
/// `a_clean_one`.
fn by_a_fouled_pool(a_clean_one: bool) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.resources.clear();
    for dx in -1..=1 {
        for dy in -1..=1 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(FOUL.0 + dx, FOUL.1 + dy)) {
                tile.soil.fouling = Soil::AS_FOUL_AS_IT_GETS;
            }
        }
    }
    world.put_a_node_down(ResourceNode::new(ResourceType::Water, Position::new(FOUL.0, FOUL.1), 100));
    if a_clean_one {
        world.put_a_node_down(ResourceNode::new(ResourceType::Water, Position::new(CLEAN.0, CLEAN.1), 100));
    }
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(30);
    agent.state.position = (HERE.0, HERE.1, 0);
    simulation
}

fn where_they_would_drink(simulation: &Simulation) -> (i32, i32) {
    let (node, _) = simulation
        .the_node_this_gather_takes(0, ResourceType::Water)
        .expect("there is water");
    let at = simulation.world.resources[node].position;
    (at.x, at.y)
}

/// Somebody who knows to passes by the fouled pool for the clean one;
/// somebody who does not drinks at the nearest.
#[test]
fn somebody_who_knows_passes_by_foul_water_for_clean() {
    let mut simulation = by_a_fouled_pool(true);
    assert_eq!(where_they_would_drink(&simulation), FOUL, "the nearest, to somebody who does not know");
    simulation.population.agents[0].found_out_how_to("know:clean_water");
    assert_eq!(where_they_would_drink(&simulation), CLEAN);
}

/// With nothing clean within reach, a drink is a drink.
#[test]
fn with_nothing_clean_a_drink_is_a_drink() {
    let mut simulation = by_a_fouled_pool(false);
    simulation.population.agents[0].found_out_how_to("know:clean_water");
    assert_eq!(where_they_would_drink(&simulation), FOUL);
}

/// Laid up from foul water twice, and somebody chooses where they drink.
#[test]
fn being_laid_up_from_foul_water_twice_is_how_it_is_found_out() {
    let mut simulation = by_a_fouled_pool(true);
    let agent = &mut simulation.population.agents[0];
    agent.taken_ill_with(crate::agents::Agent::OFF_FOUL_WATER, 0.3, 0);
    agent.what_filth_teaches();
    assert!(!agent.knows_the_way("clean_water"), "once is bad luck");
    agent.state.ailing = None;
    agent.taken_ill_with(crate::agents::Agent::OFF_FOUL_WATER, 0.3, 1_000);
    agent.what_filth_teaches();
    assert!(agent.knows_the_way("clean_water"));
}
