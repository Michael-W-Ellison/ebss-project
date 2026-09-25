// src/analytics/tests/through_the_gap_tests.rs
//! What stood between a starving settlement and its own larder.
//!
//! Every settlement starved in months ten to twelve with thousands of items
//! still in its pits. Traced person by person, besides the rot in their packs
//! (`full_pack_tests::a_pack_full_of_rot_makes_room_for_the_larder`), two
//! things kept a body from the food: the shelter override took the turns of a
//! hungry man with supper in his pack, and a half-built burrow was a wall. See
//! ISSUES_FOUND #252.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::exposure::ExposureType;
use crate::environment::Action;
use crate::world::{Building, BuildingType, Position, World, WorldConfig};

/// One person, cold, with a roof beside them and a day's roots in the pack.
fn cold_with_supper_and_a_roof(hungry: bool) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    let here = simulation.population.agents[0].state.position;
    simulation
        .world
        .buildings
        .push(Building::new(BuildingType::Burrow, Position::new(here.0 + 1, here.1)));

    let mut roots = InventoryItem::new_with_weight("roots".to_string(), 12, 1.0);
    roots.food_data = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Roots, 0);
    let agent = &mut simulation.population.agents[0];
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.get_all_items_mut().insert("roots".to_string(), roots);
    agent
        .exposure_status
        .active_exposures
        .push(ExposureType::Hypothermia);
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = if hungry { 0.9 } else { 0.0 };
    }

    simulation
}

/// Somebody hungry eats before they huddle.
///
/// The shelter override fires on being cold at all, and through the hungry
/// gap everybody is cold every turn - so it took every turn a body had,
/// including from somebody with supper in the pack, until the last quarter of
/// the reserve. Eating is one turn, and it can be done under a roof.
#[test]
fn somebody_cold_and_hungry_eats_what_they_carry_first() {
    let simulation = cold_with_supper_and_a_roof(true);
    let agent = simulation.population.agents[0].clone();
    assert!(agent.needs_shelter(), "the fixture wants somebody cold");
    assert!(
        simulation.nearest_shelter_from(agent.state.position).is_some(),
        "and a roof within reach"
    );

    let (action, _) = simulation.generate_non_emotional_action(&agent, agent.state.position);
    assert!(
        matches!(action, Action::Eat { .. }),
        "cold and hungry with supper in the pack, they were sent to {action:?}"
    );
}

/// And somebody cold and fed still goes in out of the cold: this does not
/// narrow the shelter rule, it puts a meal in front of it.
#[test]
fn somebody_cold_and_fed_still_goes_to_the_roof() {
    let simulation = cold_with_supper_and_a_roof(false);
    let agent = simulation.population.agents[0].clone();

    let (action, _) = simulation.generate_non_emotional_action(&agent, agent.state.position);
    assert!(
        matches!(action, Action::SeekShelter),
        "cold and fed, they were sent to {action:?} rather than the roof"
    );
}

/// A half-built burrow is a hole you can climb across.
///
/// A site used to block, as scaffolding. Traced through a winter: somebody on
/// a hillside between three burrows begun and never finished, and a river on
/// the fourth side, refused a step 14,560 times towards a larder eight paces
/// off.
#[test]
fn a_building_site_is_not_a_wall() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    let here = (10, 10, 0);
    simulation.population.agents[0].state.position = here;
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let at = Position::new(here.0 + dx, here.1 + dy);
        if let Some(tile) = simulation.world.grid.get_tile_mut(&at) {
            tile.terrain = crate::world::Terrain::new(crate::world::TerrainType::Meadow);
        }
        simulation
            .world
            .buildings
            .push(Building::new_under_construction(BuildingType::Burrow, at));
    }

    assert!(simulation.is_passable_tile(here.0 + 1, here.1));

    let result = simulation.execute_action(
        &Action::Move {
            target: (here.0 + 5, here.1, 0),
        },
        0,
    );
    assert!(result.success, "boxed in by building sites: {:?}", result.message);
    assert_ne!(simulation.population.agents[0].state.position, here, "and they did not move");
}

/// Nobody tastes a strange plant who could not survive the worst of one.
///
/// Eighteen people in twelve settlement-years died of a mouthful, because
/// nothing asked what condition the taster was in and a winter leaves
/// everybody in poor condition.
#[test]
fn the_worst_plant_leaves_a_taster_standing() {
    let (_, worst) = Simulation::WHAT_A_BAD_PLANT_DOES;
    assert!(
        Simulation::WELL_ENOUGH_TO_RISK_IT - worst > 0.0,
        "a taster at {} health is killed by a plant doing {worst}",
        Simulation::WELL_ENOUGH_TO_RISK_IT
    );

    let mut simulation = cold_with_supper_and_a_roof(false);
    simulation.population.agents[0].state.health = Simulation::WELL_ENOUGH_TO_RISK_IT - 1.0;
    let agent = simulation.population.agents[0].clone();
    assert!(
        simulation.tasting_action(&agent, agent.state.position).is_none(),
        "somebody under the line was let near a strange plant"
    );
}
