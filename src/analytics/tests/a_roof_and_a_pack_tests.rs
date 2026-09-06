// src/analytics/tests/a_roof_and_a_pack_tests.rs
//! Finishing a roof, the store under it, and what a pack is actually for.
//!
//! "Agents should start by carrying some food on them, then by storing some in
//! their tent, and finally by storing extra in the pit. ... Not every agent
//! need be a hunter, but all should be producing things of survival value."
//!
//! Three things were in the way of the middle rung, and all three are the same
//! defect wearing different hats: two places answering one question and never
//! being compared.
//!
//! - Nothing in the decision layer could **finish** a roof. `Build` pushed an
//!   under-construction site and the only caller of
//!   `add_construction_progress` anywhere is in the parallel world action
//!   system this layer does not issue. Every burrow ever dug was still going
//!   up when the last of the diggers died.
//! - A roof already up was read as "there is a building here, so stop", which
//!   was right while nothing could finish one and became exactly wrong the
//!   moment something could.
//! - A **working stock** was twelve items, and a pack holds 17.4 units of
//!   weight. Twelve wood at two units each is twenty-four - more than the
//!   whole pack. Nothing capped hoarding against the pack it went in, so 89.2%
//!   of packs had under five units of room and no material for any tool could
//!   ever be picked up.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::environment::Action;
use crate::world::{BuildingType, Building, Position, World, WorldConfig};

/// One person standing on their own half-dug burrow.
fn somebody_at_a_half_dug_burrow() -> crate::analytics::Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (10, 10, 0);

    // A fresh world is seeded with roofs of its own, and this fixture is about
    // one particular roof.
    simulation.world.buildings.clear();
    simulation.world.buildings.push(Building::new_under_construction(
        BuildingType::Burrow,
        Position::new(10, 10),
    ));
    simulation
}

// --------------------------------------------------------------------------
// Finishing it
// --------------------------------------------------------------------------

/// A turn of building puts work into the roof that is already up.
#[test]
fn a_turn_of_work_goes_into_the_roof_that_is_already_going_up() {
    let mut simulation = somebody_at_a_half_dug_burrow();

    let before = simulation.world.buildings[0].construction_progress();

    let result = simulation.execute_action(
        &Action::Build {
            structure_type: "burrow".to_string(),
            position: (10, 10, 0),
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);
    assert_eq!(
        simulation.world.buildings.len(),
        1,
        "he carried on with the one that was there rather than starting another"
    );
    assert!(
        simulation.world.buildings[0].construction_progress() > before,
        "and it is further up than it was"
    );
}

/// And enough turns of it finish the thing.
#[test]
fn a_roof_gets_finished_in_the_end() {
    let mut simulation = somebody_at_a_half_dug_burrow();

    for _ in 0..12 {
        let _ = simulation.execute_action(
            &Action::Build {
                structure_type: "burrow".to_string(),
                position: (10, 10, 0),
            },
            0,
        );
    }

    assert!(
        simulation.world.buildings[0].is_completed(),
        "twelve turns of digging is a burrow: {:.0}% up",
        simulation.world.buildings[0].construction_progress() * 100.0
    );
}

/// A finished roof has a store under it, and the man who finished it knows.
#[test]
fn a_finished_roof_has_a_store_under_it() {
    let mut simulation = somebody_at_a_half_dug_burrow();

    assert!(
        simulation.world.pits.is_empty(),
        "nothing is under it while it is going up"
    );

    for _ in 0..12 {
        let _ = simulation.execute_action(
            &Action::Build {
                structure_type: "burrow".to_string(),
                position: (10, 10, 0),
            },
            0,
        );
    }

    assert!(
        simulation.world.pits.iter().any(|pit| pit.where_it_is == Position::new(10, 10)),
        "the middle rung of the ladder: a hole under the floor"
    );
    assert!(
        simulation.population.agents[0]
            .memory
            .recall_locations(crate::core::memory::SpatialMemoryType::Storage)
            .iter()
            .any(|place| (place.position.0, place.position.1) == (10, 10)),
        "and the man who dug it knows where it is"
    );
}

/// A roof half up is a reason to go back to it, not a reason to stop.
#[test]
fn a_half_dug_burrow_is_a_reason_to_go_back() {
    let mut simulation = somebody_at_a_half_dug_burrow();

    // Something to dig with, since digging in wants one.
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new("handaxe".to_string(), 1));

    let answer = simulation.digging_in(&simulation.population.agents[0], (10, 10, 0));

    assert!(
        matches!(answer, Some(Action::Build { .. })),
        "he is standing on his own half-dug burrow: {answer:?}"
    );
}

/// A roof that is finished is a reason to stop, though.
#[test]
fn a_finished_roof_is_not_dug_twice() {
    let mut simulation = somebody_at_a_half_dug_burrow();
    simulation.world.buildings[0] = Building::new(BuildingType::Burrow, Position::new(10, 10));

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new("handaxe".to_string(), 1));

    assert!(
        simulation
            .digging_in(&simulation.population.agents[0], (10, 10, 0))
            .is_none(),
        "there is already a roof here"
    );
}

// --------------------------------------------------------------------------
// What a pack is for
// --------------------------------------------------------------------------

/// A working stock is a share of the pack, not a count of things.
#[test]
fn a_working_stock_is_something_a_pack_can_hold() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);

    let agent = &simulation.population.agents[0];
    let stock = crate::analytics::Simulation::what_a_working_stock_weighs(agent);

    assert!(
        stock < agent.inventory.max_weight,
        "a stock that will not go in the pack is not a stock: {stock} against {}",
        agent.inventory.max_weight
    );
    assert!(
        stock > 0.0,
        "and a man may carry something for later: {stock}"
    );
}

/// Iron is not worth carrying to somebody who cannot smelt it.
#[test]
fn nobody_carries_what_they_have_no_use_for() {
    let nobody_knows_anything = |_: &crate::environment::making::Making| false;

    assert!(
        !crate::environment::making::is_this_any_use_to("iron", &nobody_knows_anything),
        "eight units of weight, and not one step he knows takes it"
    );
}

/// And what everybody is born knowing a use for is worth carrying.
#[test]
fn what_there_is_a_use_for_is_worth_carrying() {
    let born_knowing = |step: &crate::environment::making::Making| step.obvious;

    assert!(
        crate::environment::making::is_this_any_use_to("stone", &born_knowing),
        "two stone is a knapped tip, and everybody is born knowing it"
    );
    assert!(
        crate::environment::making::is_this_any_use_to("flax", &born_knowing),
        "and flax is cordage"
    );
}

/// A thing nothing is made of is not refused - the question does not apply.
#[test]
fn the_question_does_not_apply_to_supper() {
    let nobody_knows_anything = |_: &crate::environment::making::Making| false;

    assert!(
        crate::environment::making::is_this_any_use_to("roots", &nobody_knows_anything),
        "a gate that refused everything it had no recipe for would stop a \
         settlement eating"
    );
}
