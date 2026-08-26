// src/analytics/tests/burrow_tests.rs
//! Tests for a people with nothing to build with digging itself in.
//!
//! `shelters built` was **nought in every arm ever measured**, and it was
//! three deadlocked things in a row rather than a number that wanted tuning.
//! A tent wants eight wood and four hides. Hides come off animals and nothing
//! else. And hunting sat behind six other branches and then behind being
//! desperate on top of that, so it was never reached.
//!
//! A hole in the ground with turf over it needs none of them. It is worse than
//! a tent in every way except the one that matters: it can actually be built.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{BuildingType, Position, TerrainType, World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    // Something to dig with, which is all a burrow asks for.
    {
        let digger = crate::environment::making::what_helps_with(crate::agents::SkillType::Mining)
            .next()
            .expect("something in this world digs");
        simulation.population.agents[0]
            .inventory
            .add_item(InventoryItem::new_with_weight(digger.called.to_string(), 1, 1.0));
    }
    simulation.population.agents[0].inventory.recalculate_weight();

    // Ground a hole will go in, so these tests are about the decision rather
    // than about where the world happened to put a lake.
    for dy in -4..=4 {
        for dx in -4..=4 {
            let there = Position::new(25 + dx, 25 + dy);
            if let Some(tile) = simulation.world.grid.get_tile_mut(&there) {
                tile.terrain.terrain_type = TerrainType::Plains;
            }
        }
    }

    simulation
}

// --------------------------------------------------------------------------
// What it costs
// --------------------------------------------------------------------------

/// Earth, and a morning. There is nothing to fetch and nothing to be short of,
/// which is the entire point of it.
#[test]
fn a_burrow_costs_nothing_but_the_digging() {
    assert!(
        BuildingType::Burrow.requirements().is_empty(),
        "the whole point is that a people with nothing can put one up"
    );
    assert!(
        BuildingType::Burrow.construction_time() > BuildingType::SkinTent.construction_time(),
        "a tent is put up and a burrow is dug"
    );
}

/// And a tent is still the better thing, which is why it comes first.
#[test]
fn a_tent_still_wants_wood_and_hides() {
    let wanted = BuildingType::SkinTent.requirements();
    assert!(
        !wanted.is_empty(),
        "a tent is the better shelter and costs accordingly"
    );
}

/// It is housing, so it counts as a roof for everything that asks, and it
/// needs nothing standing before it.
#[test]
fn a_burrow_is_a_roof() {
    assert!(
        BuildingType::Burrow.prerequisites().is_empty(),
        "a people with nothing has nothing standing already"
    );
    assert!(
        BuildingType::Burrow
            .functionality_description()
            .to_lowercase()
            .contains("ground"),
        "it is a hole in the ground and says so"
    );
}

// --------------------------------------------------------------------------
// Choosing to dig one
// --------------------------------------------------------------------------

/// Somebody with the makings of a tent builds a tent.
#[test]
fn somebody_who_can_build_a_tent_builds_one() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 40, 1.0));
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("hides".to_string(), 20, 1.0));
    }

    let chosen = simulation.raising_a_roof(&simulation.population.agents[0], here);

    assert!(
        matches!(
            chosen,
            Some(Action::Build { ref structure_type, .. }) if structure_type == "tent"
        ),
        "a tent is the better shelter: {chosen:?}"
    );
}

/// Somebody with wood and no hides, and nothing to hunt, digs in rather than
/// standing there. This is the deadlock that kept shelters at nought.
#[test]
fn somebody_with_no_hides_and_nothing_to_hunt_digs_in() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 40, 1.0));
    }

    let chosen = simulation.raising_a_roof(&simulation.population.agents[0], here);

    assert!(
        matches!(
            chosen,
            Some(Action::Build { ref structure_type, .. }) if structure_type == "burrow"
        ),
        "no hides, nothing to hunt, and ground that will take a hole: {chosen:?}"
    );
}

/// And nobody digs a second one on top of the first.
#[test]
fn nobody_digs_a_burrow_on_top_of_a_burrow() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 40, 1.0));
    }

    simulation
        .world
        .add_building_at(BuildingType::Burrow, here);

    let chosen = simulation.raising_a_roof(&simulation.population.agents[0], here);

    assert!(
        !matches!(
            chosen,
            Some(Action::Build { ref structure_type, .. }) if structure_type == "burrow"
        ),
        "there is already a hole here: {chosen:?}"
    );
}

/// Nor in a lake.
#[test]
fn nobody_digs_a_burrow_in_a_lake() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    for dy in -4..=4 {
        for dx in -4..=4 {
            let there = Position::new(here.0 + dx, here.1 + dy);
            if let Some(tile) = simulation.world.grid.get_tile_mut(&there) {
                tile.terrain.terrain_type = TerrainType::Water;
            }
        }
    }

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 40, 1.0));
    }

    let chosen = simulation.raising_a_roof(&simulation.population.agents[0], here);

    assert!(
        !matches!(
            chosen,
            Some(Action::Build { ref structure_type, .. }) if structure_type == "burrow"
        ),
        "that is a lake: {chosen:?}"
    );
}

// --------------------------------------------------------------------------
// Actually digging it
// --------------------------------------------------------------------------

/// The executor puts one up, and asks for nothing to do it with.
#[test]
fn digging_one_asks_for_nothing() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    let before = simulation.world.buildings.len();

    let result = simulation.execute_action(
        &Action::Build {
            structure_type: "burrow".to_string(),
            position: here,
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);
    assert!(
        simulation.world.buildings.len() > before,
        "and there should be a hole in the ground to show for it"
    );
    assert!(
        simulation
            .world
            .buildings
            .iter()
            .any(|building| building.building_type == BuildingType::Burrow),
        "a burrow, specifically"
    );
}

/// Where a tent, asked for with nothing to build it from, is refused.
#[test]
fn a_tent_with_nothing_to_build_it_from_is_refused() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    let result = simulation.execute_action(
        &Action::Build {
            structure_type: "tent".to_string(),
            position: here,
        },
        0,
    );

    assert!(!result.success, "eight wood does not come from nowhere");
}
