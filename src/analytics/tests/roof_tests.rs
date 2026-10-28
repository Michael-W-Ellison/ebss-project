// src/analytics/tests/roof_tests.rs
//! Tents and houses (#371): a roof of one's own, put up over many trips.
//!
//! A tent's skins come one kill at a time and a house is thirty poles, so
//! neither goes up from one pack. The frame is started with what its builder
//! carries; the rest is owed, and brought to the site by the household.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{BuildingType, Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// One grown person at (100, 100) on open plains, with a pack that will
/// take a load and nothing in it but what the test gives.
fn somebody_on_open_ground() -> Simulation {
    let mut world = World::new(WorldConfig { size: (200, 200), ..WorldConfig::default() });
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.resources.clear();
    for x in 0..world.grid.width as i32 {
        for y in 0..world.grid.height as i32 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain.terrain_type = TerrainType::Plains;
            }
        }
    }
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(25);
    agent.state.position = (100, 100, 0);
    agent.hearth = Some((100, 100));
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.max_weight = 500.0;
    simulation
}

fn give(simulation: &mut Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 1.0));
}

/// The household's roof of this kind, finished or not.
fn the_roof(simulation: &Simulation, kind: BuildingType) -> Option<&crate::world::Building> {
    simulation.world.buildings.iter().find(|roof| roof.building_type == kind)
}

/// Build where the roof is, as many times as it takes, or until it refuses.
fn work_on_it(simulation: &mut Simulation, kind: BuildingType, called: &str) -> bool {
    let at = the_roof(simulation, kind).expect("a site").position;
    simulation.population.agents[0].state.position = (at.x, at.y, 0);
    for _ in 0..40 {
        let result = simulation.execute_action(
            &Action::Build { structure_type: called.to_string(), position: (at.x, at.y, 0) },
            0,
        );
        if !result.success {
            return false;
        }
        if the_roof(simulation, kind).is_some_and(|roof| roof.is_completed()) {
            return true;
        }
    }
    false
}

/// A tent's frame goes up with the poles and cord in hand, and owes its
/// skins; nothing is done on it until they come, and they can come one at a
/// time.
#[test]
fn a_tent_goes_up_on_its_frame_and_waits_for_its_skins() {
    let mut simulation = somebody_on_open_ground();
    give(&mut simulation, "wood", 8);
    give(&mut simulation, "lashing", 2);
    give(&mut simulation, "hides", 1);

    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.the_roof_this_one_wants(&agent), Some(BuildingType::SkinTent));
    let doing = simulation.raising_a_roof(&agent, (100, 100, 0)).expect("something to do");
    assert!(
        matches!(&doing, Action::Build { structure_type, .. } if structure_type == "tent"),
        "the frame is in hand, and one skin of four: start the tent: {doing:?}"
    );
    let started = simulation.execute_action(&doing, 0);
    assert!(started.success, "{:?}", started.message);

    let tent = the_roof(&simulation, BuildingType::SkinTent).expect("a tent site");
    assert_eq!(tent.belongs(), crate::world::Belongs::To(agent.id), "it is his");
    assert_eq!(
        tent.still_wants(),
        &[(crate::environment::tags::Tag::FlexibleCovering, 3)],
        "the skin in hand went on with the frame, and it owes three more"
    );
    assert!(!work_on_it(&mut simulation, BuildingType::SkinTent, "tent"), "no work without skins");

    // Two skins from one hunt, two leather from another
    give(&mut simulation, "hides", 2);
    let agent = simulation.population.agents[0].clone();
    let at = the_roof(&simulation, BuildingType::SkinTent).unwrap().position;
    simulation.population.agents[0].state.position = (at.x + 10, at.y, 0);
    let agent_far = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.raising_a_roof(&agent_far, (at.x + 10, at.y, 0)),
        Some(Action::Move { target: (at.x, at.y, 0) }),
        "skins in hand: take them to the tent"
    );
    drop(agent);
    simulation.population.agents[0].state.position = (at.x, at.y, 0);
    let brought = simulation.execute_action(
        &Action::Build { structure_type: "tent".to_string(), position: (at.x, at.y, 0) },
        0,
    );
    assert!(brought.success, "{:?}", brought.message);
    assert_eq!(simulation.population.agents[0].how_many_i_have("hides"), 0);
    assert_eq!(
        the_roof(&simulation, BuildingType::SkinTent).unwrap().still_wants(),
        &[(crate::environment::tags::Tag::FlexibleCovering, 1)]
    );

    give(&mut simulation, "leather", 1);
    assert!(work_on_it(&mut simulation, BuildingType::SkinTent, "tent"), "covered, it goes up");

    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.the_roof_this_one_wants(&agent), None, "and he has a roof of his own");
}

/// A household that farms here wants a house: a framed hut of poles and
/// cord, started at its hearth with what one pack holds and finished over
/// many trips. It claims a plot and lays out ground for more houses.
#[test]
fn a_household_that_farms_here_puts_up_a_house_over_many_trips() {
    let mut simulation = somebody_on_open_ground();

    // Grain standing on a field by the hearth
    let field = Position::new(103, 100);
    if let Some(tile) = simulation.world.grid.get_tile_mut(&field) {
        tile.terrain.terrain_type = TerrainType::Farmland;
    }
    simulation.world.resources.push(ResourceNode::new(ResourceType::Grain, field, 40));

    let agent = simulation.population.agents[0].clone();
    assert!(simulation.tied_to_this_ground(&agent), "the fixture farms");
    assert_eq!(simulation.the_roof_this_one_wants(&agent), Some(BuildingType::SmallHouse));
    simulation.read_the_situation();
    assert_eq!(
        simulation.population.agents[0].surroundings.without_a_roof_of_its_own,
        Simulation::WANTING_A_HOUSE,
        "and the Construction drive is told so"
    );

    // Short of the frame: poles first
    assert_eq!(
        simulation.raising_a_roof(&agent, (100, 100, 0)),
        Some(Action::Gather { resource_type: "wood".to_string() })
    );

    give(&mut simulation, "wood", 8);
    give(&mut simulation, "lashing", 2);
    let agent = simulation.population.agents[0].clone();
    let doing = simulation.raising_a_roof(&agent, (100, 100, 0)).expect("start it");
    assert!(
        matches!(&doing, Action::Build { structure_type, position } if structure_type == "smallhouse" && (position.0, position.1) == (100, 100)),
        "the frame, at the hearth: {doing:?}"
    );
    let started = simulation.execute_action(&doing, 0);
    assert!(started.success, "{:?}", started.message);

    let house = the_roof(&simulation, BuildingType::SmallHouse).expect("a house site");
    let at = house.position;
    assert!((at.x - 100).abs().max((at.y - 100).abs()) <= 30, "near the hearth: {at:?}");
    assert!(!house.still_wants().is_empty(), "and it owes the walls and the roof");
    assert!(
        simulation.world.territory_manager.get_territory_bonus((at.x, at.y, 0), agent.id) > 0.0,
        "the plot round it is his"
    );
    assert!(
        !simulation.world.zone_manager.get_zones_at_position((at.x, at.y, 0)).is_empty(),
        "and the ground round it is laid out for houses"
    );

    // Its own site, owed poles he has not got: he goes for wood
    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.raising_a_roof(&agent, (100, 100, 0)),
        Some(Action::Gather { resource_type: "wood".to_string() })
    );

    // A pack at a time
    let mut trips = 0;
    while !the_roof(&simulation, BuildingType::SmallHouse).unwrap().still_wants().is_empty() {
        trips += 1;
        assert!(trips < 10, "it should be done in a handful of trips");
        give(&mut simulation, "wood", 8);
        give(&mut simulation, "lashing", 2);
        simulation.population.agents[0].state.position = (at.x, at.y, 0);
        let brought = simulation.execute_action(
            &Action::Build { structure_type: "smallhouse".to_string(), position: (at.x, at.y, 0) },
            0,
        );
        assert!(brought.success, "{:?}", brought.message);
    }
    assert!(trips >= 3, "thirty poles are more than one pack: {trips} trips");
    assert!(work_on_it(&mut simulation, BuildingType::SmallHouse, "smallhouse"), "and then it goes up");

    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.the_roof_this_one_wants(&agent), None, "a house of their own");
    assert!(
        BuildingType::SmallHouse.cover().warmth > BuildingType::SkinTent.cover().warmth,
        "and warmer than a tent"
    );
}

/// A house is a framed hut: no stone, which no stone-age people carries.
#[test]
fn a_house_wants_no_stone() {
    let wants = crate::environment::tags::what_this_roof_takes(BuildingType::SmallHouse)
        .expect("stated by class");
    assert!(wants.iter().all(|want| want.is != crate::environment::tags::Tag::RigidBuildingMaterial));
    assert!(!BuildingType::SmallHouse
        .requirements()
        .iter()
        .any(|r| r.resource_type == ResourceType::Stone));
}
