// src/analytics/tests/ground_tests.rs
//! Tests for things lying on the ground.
//!
//! Before this a thing was either in somebody's pack or it did not exist.
//! Nothing could be put down and picked up again, and when a person died
//! everything they had carried went out of the world with them — so an axe was
//! a thing that existed for exactly as long as its owner did, and a people that
//! spent a season making them had nothing to show for it the morning after the
//! man who made them drowned.
//!
//! A pack falls where its owner does. What is left lies there until the weather
//! has it, and anybody walking past can stoop for it.

use crate::agents::{AgentConfig, InventoryItem, Population, Quality};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Position, World, WorldConfig};

fn a_person_at(where_it_is: Position) -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (where_it_is.x, where_it_is.y, 0);

    let everything: Vec<(String, u32)> = simulation.population.agents[0]
        .inventory
        .get_all_items()
        .values()
        .map(|item| (item.item_id.clone(), item.quantity))
        .collect();

    for (what, how_many) in everything {
        for _ in 0..how_many {
            simulation.population.agents[0]
                .inventory
                .remove_item(&what, 1);
        }
    }

    simulation
}

fn give(simulation: &mut Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            what.to_string(),
            how_many,
            1.0,
        ));
}

// --------------------------------------------------------------------------
// Putting a thing down and taking it up again
// --------------------------------------------------------------------------

/// A thing put down is on the ground, and a thing picked up is in the pack.
#[test]
fn a_thing_put_down_can_be_picked_up_again() {
    let here = Position::new(25, 25);
    let mut simulation = a_person_at(here);
    give(&mut simulation, "wood", 5);

    let down = simulation.execute_action(
        &Action::PutDown {
            what: "wood".to_string(),
        },
        0,
    );
    assert!(down.success, "you can put a stick down: {:?}", down.message);
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("wood"),
        0,
        "and it is not in the pack any more"
    );
    assert_eq!(
        simulation.world.what_is_lying_at(&here).len(),
        1,
        "it is on the ground"
    );

    let up = simulation.execute_action(
        &Action::PickUp {
            what: "wood".to_string(),
        },
        0,
    );
    assert!(up.success, "and you can pick it up: {:?}", up.message);
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("wood"),
        5,
        "all of it"
    );
    assert!(
        simulation.world.what_is_lying_at(&here).is_empty(),
        "and the ground is clear again"
    );
}

/// A thing keeps what it was. A worn axe on the ground is still a worn axe.
#[test]
fn a_thing_on_the_ground_is_the_thing_it_was() {
    let here = Position::new(25, 25);
    let mut simulation = a_person_at(here);

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_durability(
            "handaxe".to_string(),
            1,
            40.0,
            Quality::Advanced,
        ));

    // Worn most of the way through
    if let Some(axe) = simulation.population.agents[0]
        .inventory
        .get_item_mut("handaxe")
    {
        axe.current_durability = Some(6.0);
    }

    simulation.execute_action(
        &Action::PutDown {
            what: "handaxe".to_string(),
        },
        0,
    );
    simulation.execute_action(
        &Action::PickUp {
            what: "handaxe".to_string(),
        },
        0,
    );

    let axe = simulation.population.agents[0]
        .inventory
        .get_item("handaxe")
        .expect("it came back");

    assert_eq!(
        axe.current_durability,
        Some(6.0),
        "as worn as it was when it went down"
    );
    assert_eq!(axe.quality, Some(Quality::Advanced), "and as well made");
}

/// Nothing lying here is nothing to pick up.
#[test]
fn nothing_lying_there_is_nothing_to_pick_up() {
    let here = Position::new(25, 25);
    let mut simulation = a_person_at(here);

    let result = simulation.execute_action(
        &Action::PickUp {
            what: "wood".to_string(),
        },
        0,
    );
    assert!(!result.success, "there is no stick there");
}

/// And a pack with no room in it leaves the thing where it was.
#[test]
fn a_full_pack_leaves_it_on_the_ground() {
    let here = Position::new(25, 25);
    let mut simulation = a_person_at(here);

    simulation.world.somebody_left_this(
        InventoryItem::new_with_weight("stone".to_string(), 40, 5.0),
        here,
        0,
    );

    // Loaded to the limit
    let room = simulation.population.agents[0]
        .inventory
        .weight_capacity_remaining();
    give(&mut simulation, "wood", room.ceil() as u32);

    let result = simulation.execute_action(
        &Action::PickUp {
            what: "stone".to_string(),
        },
        0,
    );

    assert!(!result.success, "there is nowhere to put it");
    assert_eq!(
        simulation.world.what_is_lying_at(&here).len(),
        1,
        "and it is still lying there, not vanished"
    );
}

// --------------------------------------------------------------------------
// What the dead leave
// --------------------------------------------------------------------------

/// A pack falls where its owner does.
#[test]
fn what_somebody_was_carrying_stays_where_they_fell() {
    let here = Position::new(25, 25);
    let mut simulation = a_person_at(here);

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_durability(
            "spear".to_string(),
            1,
            25.0,
            Quality::Basic,
        ));
    give(&mut simulation, "flint", 4);

    simulation.population.agents[0].state.is_alive = false;
    simulation.population.agents[0].state.health = 0.0;

    // A tick, so the population clears its dead and the world takes what
    // they were carrying
    simulation.tick();

    let left: Vec<String> = simulation
        .world
        .what_is_lying_at(&here)
        .into_iter()
        .map(|left| left.item.item_id.clone())
        .collect();

    assert!(
        left.contains(&"spear".to_string()),
        "his spear is where he fell: {left:?}"
    );
    assert!(
        left.contains(&"flint".to_string()),
        "and so are his flakes: {left:?}"
    );
}

/// And the next person along picks it up.
#[test]
fn somebody_else_can_take_up_what_the_dead_left() {
    let here = Position::new(25, 25);
    let mut simulation = a_person_at(here);

    simulation.world.somebody_left_this(
        InventoryItem::new_with_durability("handaxe".to_string(), 1, 40.0, Quality::Basic),
        here,
        0,
    );

    let position = simulation.population.agents[0].state.position;
    let stooped = simulation.something_worth_stooping_for(
        &simulation.population.agents[0],
        position,
    );

    assert!(
        matches!(
            stooped,
            Some(Action::PickUp { ref what }) if what == "handaxe"
        ),
        "a man with no axe standing on one should pick it up: {stooped:?}"
    );
}

/// A thing worth having a little way off is worth the walk.
#[test]
fn a_thing_a_little_way_off_is_worth_the_walk() {
    let here = Position::new(25, 25);
    let over_there = Position::new(31, 25);
    let mut simulation = a_person_at(here);

    simulation.world.somebody_left_this(
        InventoryItem::new_with_durability("handaxe".to_string(), 1, 40.0, Quality::Basic),
        over_there,
        0,
    );

    let position = simulation.population.agents[0].state.position;

    match simulation.something_worth_stooping_for(&simulation.population.agents[0], position) {
        Some(Action::Move { target }) => {
            assert_eq!(
                (target.0, target.1),
                (over_there.x, over_there.y),
                "he goes to it"
            );
        }
        other => panic!("an axe six paces off is worth six paces: {other:?}"),
    }
}

/// But not across the country.
#[test]
fn a_thing_across_the_map_is_not() {
    let here = Position::new(25, 25);
    let mut simulation = a_person_at(here);

    simulation.world.somebody_left_this(
        InventoryItem::new_with_durability("handaxe".to_string(), 1, 40.0, Quality::Basic),
        Position::new(80, 25),
        0,
    );

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .something_worth_stooping_for(&simulation.population.agents[0], position)
            .is_none(),
        "nobody walks fifty tiles on the off-chance"
    );
}

// --------------------------------------------------------------------------
// What the weather does to it
// --------------------------------------------------------------------------

/// Food left lying goes first, and goes into the ground.
#[test]
fn food_left_lying_goes_into_the_ground() {
    let here = Position::new(25, 25);
    let mut simulation = a_person_at(here);

    let mut supper = InventoryItem::new_with_weight("food".to_string(), 10, 0.5);
    supper.food_data = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Food, 0);

    simulation.world.somebody_left_this(supper, here, 0);

    let litter_before = simulation
        .world
        .grid
        .get_tile(&here)
        .map(|tile| tile.soil.litter())
        .unwrap_or(0.0);

    for _ in 0..(World::HOW_LONG_A_THING_LIES_THERE / 2) {
        simulation.world.tick();
    }

    assert!(
        simulation.world.what_is_lying_at(&here).is_empty(),
        "a basket of berries does not keep on open ground"
    );
    assert!(
        simulation
            .world
            .grid
            .get_tile(&here)
            .map(|tile| tile.soil.litter())
            .unwrap_or(0.0)
            > litter_before,
        "and what it was is in the ground now"
    );
}

/// A stone axe keeps longer than a basket of berries, and not for ever.
#[test]
fn a_tool_keeps_longer_than_food_and_not_for_ever() {
    let here = Position::new(25, 25);
    let mut simulation = a_person_at(here);

    simulation.world.somebody_left_this(
        InventoryItem::new_with_durability("handaxe".to_string(), 1, 40.0, Quality::Basic),
        here,
        0,
    );

    for _ in 0..(World::HOW_LONG_A_THING_LIES_THERE / 2) {
        simulation.world.tick();
    }
    assert_eq!(
        simulation.world.what_is_lying_at(&here).len(),
        1,
        "still there when the berries would have gone"
    );

    for _ in 0..World::HOW_LONG_A_THING_LIES_THERE {
        simulation.world.tick();
    }
    assert!(
        simulation.world.what_is_lying_at(&here).is_empty(),
        "and gone in the end"
    );
}
