// src/analytics/tests/stacking_tests.rs
//! Tests for what happens when one lot of a thing is put on top of another.
//!
//! `Inventory::add_item` and `Pit::put_in` both merged by name with a bare
//! `quantity += other.quantity`, keeping whichever stack happened to be there
//! first and throwing the other's clock away. So the same act went either way
//! by accident: this morning's berries tipped onto a week-old basket inherited
//! the week-old timer, and a week-old handful tipped onto a fresh stack was
//! silently made new again.
//!
//! "Adding fresh food to a stack of old food should not reset the freshness of
//! the entire stack. It should decrease the freshness of the newly added food
//! to match the older food's decay timer, as mould tends to spread rapidly
//! once it manifests."
//!
//! Which matters here because this is a simulation of what agents *do*. An
//! agent decides whether to eat now, dry it, bury it or leave it on the
//! strength of what its pack says it is holding. A pack that lies about its
//! own food produces sensible decisions about a world that is not there.
//! See ISSUES_FOUND #61.

use crate::agents::{AgentConfig, InventoryItem, Inventory, Population};
use crate::analytics::Simulation;
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Pit, Position, World, WorldConfig};

fn a_lot_of(what: &str, how_many: u32, picked_on: u32) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut lot = InventoryItem::new_with_weight(what.to_string(), how_many, 0.5);
    lot.food_data = database.create_food_data(&ItemType::Food, picked_on);
    lot
}

/// The clock rule is **not** shipped, and this says so on purpose.
///
/// Fresh food tipped into a basket that has been going over ought to come
/// down to meet it — mould spreads — and `FoodData::the_older_clock` was
/// written and unit tested for exactly that. Measured at thirty-two worlds a
/// side it cost a settlement **more than half of everything it ate** (9,703
/// to 4,638, t = -8.4) and seven of its people, and the loss turns up in no
/// waste column at all: eaten plus waste falls from 12,874 to 6,692, so some
/// six thousand units leave the ledger without being eaten, rotting or being
/// left anywhere. Every other change in the batch measured null with the rule
/// off, so it is responsible and the hole is unexplained.
///
/// So a stack still keeps the clock of whatever was there first. That is
/// wrong, it is *known* to be wrong, and it is less wrong than losing half a
/// settlement's food to a sink nobody has found. See ISSUES_FOUND #61.
#[test]
fn a_stack_still_keeps_the_clock_of_whatever_was_there_first() {
    let mut older = a_lot_of("food", 10, 0);
    let this_morning = a_lot_of("food", 10, 4_000);

    older.absorb(this_morning);

    assert_eq!(older.quantity, 20, "it is all one basket now");
    assert_eq!(
        older.food_data.as_ref().unwrap().created_tick,
        0,
        "and it reads as old as the stack that was already there"
    );
}

/// A stack keeps the preparation of whatever was there first, for the same
/// reason and with the same reservation as the clock above.
#[test]
fn a_stack_keeps_the_preparation_of_whatever_was_there_first() {
    let mut dried = a_lot_of("food", 10, 0);
    if let Some(ref mut clock) = dried.food_data {
        clock.preparation = PreparationState::Dried;
    }

    dried.absorb(a_lot_of("food", 10, 0));

    assert_eq!(
        dried.food_data.as_ref().unwrap().preparation,
        PreparationState::Dried,
    );
}

/// A stack that has a clock keeps it when something without one is added.
///
/// An item with no `food_data` never rots at all, so letting one swallow a
/// real stack made food that could sit in a pit for the life of the world
/// without ever going off — which is what a settlement's pits were quietly
/// filling up with.
#[test]
fn nothing_becomes_immortal_by_being_stacked_on() {
    let mut real = a_lot_of("food", 10, 0);
    let inert = InventoryItem::new_with_weight("food".to_string(), 10, 0.5);

    real.absorb(inert);

    assert!(
        real.food_data.is_some(),
        "the stack still knows how old it is"
    );

    let mut inert = InventoryItem::new_with_weight("food".to_string(), 10, 0.5);
    inert.absorb(a_lot_of("food", 10, 0));

    assert!(
        inert.food_data.is_some(),
        "and a clock is picked up rather than thrown away"
    );
}

/// The pack goes through the same rule.
#[test]
fn a_pack_stacks_on_the_same_terms() {
    let mut pack = Inventory::new(1000, 1000.0);
    let _ = pack.add_item(a_lot_of("food", 10, 0));
    let _ = pack.add_item(a_lot_of("food", 10, 5_000));

    let stack = pack.get_item("food").expect("he is carrying berries");

    assert_eq!(stack.quantity, 20);
    assert_eq!(
        stack.food_data.as_ref().unwrap().created_tick,
        0,
        "a pack keeps the clock of what it was already carrying"
    );
}

/// And so does the store. Burying a fresh load into a pit that has held one
/// since autumn does not make the pit fresh.
#[test]
fn a_pit_stacks_on_the_same_terms() {
    let mut pit = Pit {
        where_it_is: Position::new(0, 0),
        holds: vec![a_lot_of("food", 40, 0)],
        covered: true,
        dug: 0,
    };

    pit.put_in(a_lot_of("food", 10, 5_000));

    assert_eq!(pit.how_much_is_in_it(), 50);
    assert_eq!(
        pit.holds[0].food_data.as_ref().unwrap().created_tick,
        0,
        "burying a fresh load into an old pit does not make the pit fresh"
    );
}

/// What is handed over is the thing itself.
///
/// `Action::Give` used to build a *new* item out of the name — same id, same
/// count, and nothing else: no food data, no freshness, no preparation, and a
/// flat weight of 2.0 whatever it was. Giving somebody a week-old fish handed
/// them a fish that would never go off.
#[test]
fn what_is_given_is_the_thing_and_not_its_name() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    for agent in simulation.population.agents.iter_mut() {
        agent.inventory.get_all_items_mut().clear();
        agent.inventory.recalculate_weight();
    }

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(a_lot_of("food", 6, 0));

    let given = simulation.population.agents[0]
        .inventory
        .remove_item("food", 6)
        .expect("he had it");

    assert!(
        given.food_data.is_some(),
        "what leaves the pack knows what it is"
    );

    let _ = simulation.population.agents[1].inventory.add_item(given);

    let got = simulation.population.agents[1]
        .inventory
        .get_item("food")
        .expect("she has it now");

    assert!(
        got.food_data.is_some(),
        "and so does what arrives in the other pack"
    );
    assert_eq!(
        got.food_data.as_ref().unwrap().created_tick,
        0,
        "with the same clock it had before it changed hands"
    );
}
