// src/analytics/tests/tubers_tests.rs
//! Propagation from tubers (ISSUES_FOUND #384).
//!
//! "Propagation from cuttings, tubers, and saved seed", stage 2 of the
//! agricultural path. Cuttings and saved seed were in; roots could be neither
//! sown nor lifted. Growing them began with the top of a dug tuber put back
//! in the hole it came out of.
//!
//! - a piece of tuber is lifted from a stand of roots far from home
//! - somebody breaking ground with nothing but roots in the pack puts roots in

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, Terrain, TerrainType, World, WorldConfig};

/// One grown person who gardens, at `at` on open grass with nothing about.
fn a_gardener_at(at: Position) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    for dx in -25..=25 {
        for dy in -8..=8 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(at.x + dx, at.y + dy)) {
                tile.terrain = Terrain::new(TerrainType::Plains);
            }
        }
    }
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(30);
    agent.state.position = (at.x, at.y, 0);
    agent.found_out_how_to("do:ScatterSeed");
    agent.found_out_how_to("do:TillSoil");
    agent.inventory.get_all_items_mut().clear();
    simulation
}

/// A piece of tuber comes away from a stand of roots a long way from home.
#[test]
fn a_piece_of_tuber_is_lifted_from_a_stand_of_roots() {
    let camp = Position::new(25, 25);
    let far_off = Position::new(45, 25);
    let mut simulation = a_gardener_at(camp);
    simulation.population.agents[0].hearth = Some((camp.x, camp.y));
    let mut stand = ResourceNode::new(ResourceType::Roots, far_off, 60);
    stand.amount = 60;
    simulation.world.put_a_node_down(stand);
    simulation.population.agents[0].state.position = (far_off.x, far_off.y, 0);

    let position = simulation.population.agents[0].state.position;
    assert!(
        matches!(
            simulation.transplanting_action(&simulation.population.agents[0], position),
            Some(Action::TakeCutting)
        ),
        "twenty tiles from the tents, with a stand of roots underfoot"
    );
    let result = simulation.execute_action(&Action::TakeCutting, 0);
    assert!(result.success, "{:?}", result.message);
    assert_eq!(simulation.population.agents[0].how_many_i_have("rootscutting"), 1);
}

/// Somebody breaking ground with roots in the pack and nothing else sowable
/// puts roots in.
#[test]
fn roots_are_what_go_in_when_roots_are_what_there_is() {
    let simulation = a_gardener_at(Position::new(25, 25));
    let mut agent = simulation.population.agents[0].clone();
    agent.inventory.add_item(InventoryItem::new_with_weight("roots".to_string(), 3, 1.0));
    assert_eq!(Simulation::what_this_one_would_sow(&agent, 1.0), ResourceType::Roots);
}
