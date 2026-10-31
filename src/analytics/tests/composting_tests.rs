// src/analytics/tests/composting_tests.rs
//! Compost from the midden (ISSUES_FOUND #386).
//!
//! - what goes onto a midden rots down into compost over a couple of months
//! - a gardener walks to the camp's midden and digs out what has rotted
//! - nobody digs at a camp with no garden, at a midden with nothing rotted,
//!   or before gardens are known
//! - compost goes onto a field as muck does

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Midden, Position, ResourceNode, ResourceType, Terrain, TerrainType, World, WorldConfig};

const HOME: (i32, i32) = (25, 25);
const MIDDEN: (i32, i32) = (33, 25);

/// One grown gardener at home, with the camp's midden holding `rotted` of
/// compost and, if `garden`, a crop standing on broken ground near home.
fn a_gardener(rotted: f32, garden: bool) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.resources.clear();
    world.middens.push(Midden { at: Position::new(MIDDEN.0, MIDDEN.1), started: 0, holds: 0.0, rotted });
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    if garden {
        let at = Position::new(HOME.0 - 5, HOME.1);
        if let Some(tile) = simulation.world.grid.get_tile_mut(&at) {
            tile.terrain = Terrain::new(TerrainType::Farmland);
        }
        let mut crop = ResourceNode::new(ResourceType::Grain, at, 40);
        crop.on_a_field = true;
        simulation.world.put_a_node_down(crop);
    }
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(30);
    agent.state.position = (HOME.0, HOME.1, 0);
    agent.hearth = Some(HOME);
    agent.inventory.get_all_items_mut().clear();
    agent.found_out_how_to("do:ScatterSeed");
    agent.found_out_how_to("do:TillSoil");
    simulation
}

/// What goes onto a midden rots down, most of it inside two months.
#[test]
fn a_midden_rots_down_into_compost() {
    let mut midden = Midden { at: Position::new(0, 0), started: 0, holds: 60.0, rotted: 0.0 };
    for _ in 0..60 {
        midden.a_day_rotting_down();
    }
    assert!(midden.rotted > 30.0 && midden.rotted < 45.0, "{}", midden.rotted);
    assert!((midden.rotted + midden.holds - 60.0).abs() < 0.01, "nothing lost on the way");
}

/// A gardener walks to the camp's midden, digs out what one trip carries,
/// and does not go back while carrying it.
#[test]
fn a_gardener_digs_out_the_midden_for_the_garden() {
    let mut simulation = a_gardener(10.0, true);
    let agent = simulation.population.agents[0].clone();
    let Some(Action::Move { target }) = simulation.compost_to_dig(&agent, agent.state.position) else {
        panic!("walks to the midden");
    };
    assert_eq!((target.0, target.1), MIDDEN);

    simulation.population.agents[0].state.position = (MIDDEN.0, MIDDEN.1, 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.compost_to_dig(&agent, agent.state.position), Some(Action::DigOutMidden));
    let result = simulation.execute_action(&Action::DigOutMidden, 0);
    assert!(result.success, "{result:?}");
    assert_eq!(simulation.population.agents[0].how_many_i_have("compost"), Midden::LOADS_DUG_OUT_AT_ONCE);
    assert!((simulation.world.middens[0].rotted - 6.0).abs() < 0.01);
    assert_eq!(simulation.world.compost_dug, Midden::LOADS_DUG_OUT_AT_ONCE as u64);

    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.compost_to_dig(&agent, agent.state.position), None, "not while carrying it");
}

/// No garden, nothing rotted, or no gardens known: nobody digs.
#[test]
fn nobody_digs_without_a_garden_compost_or_gardening() {
    let no_garden = a_gardener(10.0, false);
    let agent = no_garden.population.agents[0].clone();
    assert_eq!(no_garden.compost_to_dig(&agent, agent.state.position), None, "no garden");

    let nothing_rotted = a_gardener(0.5, true);
    let agent = nothing_rotted.population.agents[0].clone();
    assert_eq!(nothing_rotted.compost_to_dig(&agent, agent.state.position), None, "nothing rotted");

    let mut forager = a_gardener(10.0, true);
    forager.population.agents[0] = {
        let mut fresh = crate::agents::Agent::new(AgentConfig::default());
        fresh.state.now_this_many_years_old(30);
        fresh.state.position = (HOME.0, HOME.1, 0);
        fresh.hearth = Some(HOME);
        fresh
    };
    let agent = forager.population.agents[0].clone();
    assert_eq!(forager.compost_to_dig(&agent, agent.state.position), None, "nobody who keeps no garden");
}

/// Compost goes onto a field as muck does.
#[test]
fn compost_goes_on_a_field_as_muck() {
    let mut simulation = a_gardener(0.0, true);
    let field = (HOME.0 - 5, HOME.1);
    simulation.population.agents[0].state.position = (field.0, field.1, 0);
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("compost".to_string(), 3, 1.0));
    let result = simulation.execute_action(&Action::SpreadMuck, 0);
    assert!(result.success, "{result:?}");
    assert_eq!(simulation.population.agents[0].how_many_i_have("compost"), 0);
}
