// src/analytics/tests/transplant_tests.rs
//! Tests for moving a plant you already know is good to ground beside the
//! camp.
//!
//! "Farming could also develop from transplanting known good plants so they
//! grow closer to a camp or settlement."
//!
//! This is the way into farming that needs no seed and no theory. Somebody
//! walks half a morning to the same berry bush every day, and one day digs it
//! up and puts it in beside the tents. It is not an idea about agriculture,
//! it is an idea about the walk - and what it leaves behind is a plant growing
//! where somebody put it, which is a field in everything but name.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{
    Position, ResourceNode, ResourceType, Terrain, TerrainType, World, WorldConfig,
};

/// A camp of four with nothing growing anywhere near it
fn a_camp_at(camp: Position) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.retain(|resource| {
        !matches!(
            resource.resource_type,
            ResourceType::Food | ResourceType::Grain | ResourceType::Herbs
        )
    });

    // Open grass right round the camp, so there is somewhere to put a slip.
    // Clear it of everything else as well: a slip will not go in on top of a
    // boulder, and whether the world happened to drop one on the camp tile is
    // not what any of these tests is about.
    for dx in -8..=8 {
        for dy in -8..=8 {
            let tile_position = Position::new(camp.x + dx, camp.y + dy);
            if let Some(tile) = world.grid.get_tile_mut(&tile_position) {
                tile.terrain = Terrain::new(TerrainType::Plains);
            }
        }
    }
    world.resources.retain(|resource| {
        (resource.position.x as i32 - camp.x as i32).abs() > 8
            || (resource.position.y as i32 - camp.y as i32).abs() > 8
    });

    let mut population = Population::new();
    for _ in 0..4 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);
    for agent in &mut simulation.population.agents {
        agent.state.position = (camp.x, camp.y, 0);
    }
    simulation
}

fn put_plant(simulation: &mut Simulation, where_it_is: Position, crop: ResourceType, how_much: u32) {
    if let Some(tile) = simulation.world.grid.get_tile_mut(&where_it_is) {
        tile.terrain = Terrain::new(TerrainType::Plains);
    }
    // Whatever the world happened to put there is not what this test is
    // about, and a second resource on the tile is the difference between
    // lifting a slip of the plant the test placed and lifting a boulder
    simulation
        .world
        .resources
        .retain(|resource| resource.position != where_it_is);
    let mut patch = ResourceNode::new(crop, where_it_is, how_much.max(1));
    patch.amount = how_much;
    simulation.world.resources.push(patch);
}

// --------------------------------------------------------------------------
// Lifting a slip
// --------------------------------------------------------------------------

/// Standing at a plant a long way from home, an agent lifts a piece of it.
#[test]
fn a_plant_a_long_way_off_is_worth_lifting() {
    let camp = Position::new(25, 25);
    let far_off = Position::new(45, 25);

    let mut simulation = a_camp_at(camp);
    put_plant(&mut simulation, far_off, ResourceType::Food, 60);

    simulation.population.agents[0].state.position = (far_off.x, far_off.y, 0);
    let position = simulation.population.agents[0].state.position;

    assert!(
        matches!(
            simulation.transplanting_action(&simulation.population.agents[0], position),
            Some(Action::TakeCutting)
        ),
        "twenty tiles from the tents, with a berry bush underfoot"
    );

    let result = simulation.execute_action(&Action::TakeCutting, 0);
    assert!(result.success, "the slip comes away: {:?}", result.message);
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("foodcutting"),
        1,
        "and goes in the pack"
    );

    assert!(
        simulation
            .world
            .resources
            .iter()
            .find(|resource| resource.position == far_off)
            .map(|resource| resource.amount)
            .unwrap_or(0)
            < 60,
        "the bush it came off is the smaller for it"
    );
}

/// A bush already growing beside the tents is not worth moving anywhere.
#[test]
fn nobody_moves_a_plant_that_is_already_where_they_live() {
    let camp = Position::new(25, 25);
    let mut simulation = a_camp_at(camp);
    put_plant(&mut simulation, camp, ResourceType::Food, 60);

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .transplanting_action(&simulation.population.agents[0], position)
            .is_none(),
        "it is already where you want it"
    );
}

/// Nothing comes off a patch too thin to spare it.
#[test]
fn a_thin_patch_gives_up_no_slips() {
    let camp = Position::new(25, 25);
    let far_off = Position::new(45, 25);

    let mut simulation = a_camp_at(camp);
    put_plant(&mut simulation, far_off, ResourceType::Food, 1);
    simulation.population.agents[0].state.position = (far_off.x, far_off.y, 0);

    let result = simulation.execute_action(&Action::TakeCutting, 0);
    assert!(
        !result.success,
        "a single plant is not a patch to dig slips out of"
    );
}

// --------------------------------------------------------------------------
// Putting it in
// --------------------------------------------------------------------------

/// A carried slip goes in the ground where the agent lives, not where it was
/// picked up.
#[test]
fn a_carried_slip_gets_planted_at_home() {
    let camp = Position::new(25, 25);
    let mut simulation = a_camp_at(camp);

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            "foodcutting".to_string(),
            1,
            1.5,
        ));

    let position = simulation.population.agents[0].state.position;
    assert!(
        matches!(
            simulation.transplanting_action(&simulation.population.agents[0], position),
            Some(Action::PlantCutting)
        ),
        "standing in camp with a slip in the pack and open grass underfoot"
    );

    let result = simulation.execute_action(&Action::PlantCutting, 0);
    assert!(result.success, "it goes in: {:?}", result.message);

    let planted = simulation
        .world
        .resources
        .iter()
        .find(|resource| resource.position == camp);

    assert!(planted.is_some(), "there is a plant beside the camp now");
    assert_eq!(
        planted.map(|resource| resource.resource_type),
        Some(ResourceType::Food),
        "and it is the thing that was carried home"
    );
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("foodcutting"),
        0,
        "the pack is empty again"
    );
}

/// Carrying a slip a long way from home means walking home with it.
#[test]
fn a_slip_carried_far_from_home_gets_carried_home() {
    let camp = Position::new(25, 25);
    let far_off = Position::new(45, 25);

    let mut simulation = a_camp_at(camp);
    // The rest of the camp stays put, so the middle of the knot is still home
    simulation.population.agents[0].state.position = (far_off.x, far_off.y, 0);
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            "foodcutting".to_string(),
            1,
            1.5,
        ));

    let position = simulation.population.agents[0].state.position;

    match simulation.transplanting_action(&simulation.population.agents[0], position) {
        Some(Action::Move { target }) => {
            assert!(
                (target.0 - camp.x).abs() <= 8 && (target.1 - camp.y).abs() <= 8,
                "the slip goes home, not {target:?}"
            );
        }
        other => panic!("a slip in the pack is a reason to walk home: {other:?}"),
    }
}

/// A slip does not go in on top of something already growing.
#[test]
fn a_slip_does_not_go_in_on_top_of_a_plant() {
    let camp = Position::new(25, 25);
    let mut simulation = a_camp_at(camp);
    put_plant(&mut simulation, camp, ResourceType::Grain, 20);

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            "foodcutting".to_string(),
            1,
            1.5,
        ));

    let result = simulation.execute_action(&Action::PlantCutting, 0);
    assert!(!result.success, "that ground is taken");
}

/// And nothing takes on a mountainside.
#[test]
fn a_slip_does_not_take_on_bare_rock() {
    let camp = Position::new(25, 25);
    let mut simulation = a_camp_at(camp);

    if let Some(tile) = simulation.world.grid.get_tile_mut(&camp) {
        tile.terrain = Terrain::new(TerrainType::Mountain);
    }

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            "foodcutting".to_string(),
            1,
            1.5,
        ));

    let result = simulation.execute_action(&Action::PlantCutting, 0);
    assert!(!result.success, "nothing takes on rock");
}

// --------------------------------------------------------------------------
// What it is for
// --------------------------------------------------------------------------

/// The point of the whole thing: the walk to food gets shorter.
#[test]
fn transplanting_brings_the_food_closer() {
    let camp = Position::new(25, 25);
    let far_off = Position::new(45, 25);

    let mut simulation = a_camp_at(camp);
    put_plant(&mut simulation, far_off, ResourceType::Food, 60);

    let how_far_to_food = |simulation: &Simulation| {
        simulation
            .world
            .resources
            .iter()
            .filter(|resource| resource.resource_type == ResourceType::Food)
            .map(|resource| camp.distance_to(&resource.position))
            .min()
            .unwrap_or(u32::MAX)
    };

    let before = how_far_to_food(&simulation);

    simulation.population.agents[0].state.position = (far_off.x, far_off.y, 0);
    simulation.execute_action(&Action::TakeCutting, 0);
    simulation.population.agents[0].state.position = (camp.x, camp.y, 0);
    simulation.execute_action(&Action::PlantCutting, 0);

    let after = how_far_to_food(&simulation);

    assert!(
        after < before,
        "the walk should be shorter afterwards: {after} against {before}"
    );
}
