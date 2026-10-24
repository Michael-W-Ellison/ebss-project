// src/analytics/tests/behaviour_gaps_tests.rs
//! Behaviour that came out wrong for want of one question asked: a worn-out
//! field counted as a field (#5), a hungry man setting off for the furthest
//! food he knew (#68's neighbour in `migration_action`), a dead swap made a
//! dozen times (#69) and again every season (#364), and grain that never
//! reached a pack to be sown (#10), or was eaten when it did.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::memory::SpatialMemoryType;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// One grown person at (25, 25) on open plains a hundred and twenty wide,
/// nothing on them and nothing about.
fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default().with_size(120, 50));
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.resources.clear();
    world.pits.clear();
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
    agent.state.position = (25, 25, 0);
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.max_weight = 500.0;
    simulation
}

/// A field worked down to exhaustion is not counted towards the fields a
/// settlement wants, so a people whose ground is worn out breaks new ground.
#[test]
fn a_worn_out_field_is_not_counted() {
    let mut simulation = one_person();
    let now = simulation.world.turn;
    for dx in 0..3 {
        assert!(simulation.world.grid.break_ground(&Position::new(26 + dx, 25), now));
    }
    assert_eq!(simulation.fields_within((25, 25, 0), 5), 3);

    for dx in 0..2 {
        simulation
            .world
            .grid
            .field_at_mut(&Position::new(26 + dx, 25))
            .expect("broken ground is a field")
            .grade = crate::world::soil::SoilGrade::Exhausted;
    }
    assert_eq!(simulation.fields_within((25, 25, 0), 5), 1, "only the one still worth sowing");
}

/// Somebody leaving for want of food heads for the nearest place they
/// remember food that is far enough to be worth the walk, not the furthest.
#[test]
fn a_hungry_man_leaves_for_the_nearest_food_he_knows() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent.drives.get_mut(DriveType::Hunger).expect("hunger").denied_turns =
        Simulation::HUNGRY_ENOUGH_TO_LEAVE + 1;
    let near = 25 + Simulation::FAR_ENOUGH_TO_BE_WORTH_THE_WALK + 2;
    let far = 25 + Simulation::FAR_ENOUGH_TO_BE_WORTH_THE_WALK + 40;
    agent.memory.remember_location(SpatialMemoryType::Food, (far, 25, 0));
    agent.memory.remember_location(SpatialMemoryType::Food, (near, 25, 0));

    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.migration_action(&agent, (25, 25, 0)),
        Some(Action::Move { target: (near, 25, 0) })
    );
}

/// A swap comes out the same every time, so one tried is not tried again.
#[test]
fn a_swap_tried_is_not_tried_again() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent.found_out_how_to(crate::environment::making::THONG_FOR_CORD.instead_of_making);
    agent.inventory.add_item(InventoryItem::new("stone".to_string(), 3));
    agent.inventory.add_item(InventoryItem::new("hides".to_string(), 3));

    let first = agent.what_i_would_swap().expect("something to put where the flax goes");
    let called = crate::environment::making::what_that_swap_is_called(&first.0, &first.1, &first.2);
    agent.lessons.record_particular(&called, false);
    agent.tried_this_swap(&called);

    assert_ne!(agent.what_i_would_swap(), Some(first.clone()), "not the same swap twice");

    // And not after the lesson has faded either: years on, it is still known.
    agent.lessons.fade(10 * crate::environment::seasons::TICKS_PER_YEAR);
    assert_eq!(agent.lessons.tried_this(&called), 0, "the lesson itself is gone");
    assert_ne!(agent.what_i_would_swap(), Some(first), "but the swap is still not tried again");
}

/// The executor is what notes the swap as tried.
#[test]
fn trying_a_swap_notes_it_for_good() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent.found_out_how_to(crate::environment::making::THONG_FOR_CORD.instead_of_making);
    agent.inventory.add_item(InventoryItem::new("stone".to_string(), 3));
    let (makes, instead_of, put_in) = agent.what_i_would_swap().expect("a swap to try");

    simulation.execute_action(
        &Action::TrySwapping {
            instead_of_making: makes.clone(),
            instead_of: instead_of.clone(),
            put_in: put_in.clone(),
        },
        0,
    );

    simulation.population.agents[0].inventory.add_item(InventoryItem::new(put_in.clone(), 3));
    assert_ne!(
        simulation.population.agents[0].what_i_would_swap(),
        Some((makes, instead_of, put_in)),
        "tried once, not tried again"
    );
}

/// Somebody about to break ground with no grain on them fetches some from
/// wild grain they know of first; somebody already carrying it does not.
#[test]
fn a_farmer_fetches_grain_before_breaking_ground() {
    let mut simulation = one_person();
    let stand = Position::new(30, 25);
    simulation.world.put_a_node_down(ResourceNode::new(ResourceType::Grain, stand, 20));
    simulation.population.agents[0]
        .exploration_knowledge
        .known_resources
        .insert(stand, ResourceType::Grain);

    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.wild_grain_to_sow_from(&agent, (25, 25, 0)), Some(stand));

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new("grain".to_string(), 2));
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.wild_grain_to_sow_from(&agent, (25, 25, 0)), None);

    // And nobody goes far for it.
    let mut simulation = one_person();
    let distant = Position::new(25 + Simulation::A_WALK_FOR_SEED as i32 + 5, 25);
    simulation.world.put_a_node_down(ResourceNode::new(ResourceType::Grain, distant, 20));
    simulation.population.agents[0]
        .exploration_knowledge
        .known_resources
        .insert(distant, ResourceType::Grain);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.wild_grain_to_sow_from(&agent, (25, 25, 0)), None);
}

/// And the gathering itself puts grain in the pack.
#[test]
fn gathering_wild_grain_puts_seed_in_the_pack() {
    let mut simulation = one_person();
    simulation
        .world
        .put_a_node_down(ResourceNode::new(ResourceType::Grain, Position::new(25, 25), 20));
    let got = simulation.execute_action(&Action::Gather { resource_type: "grain".to_string() }, 0);
    assert!(got.success, "{:?}", got.message);
    assert!(simulation.population.agents[0].how_many_i_have("grain") > 0);
}
