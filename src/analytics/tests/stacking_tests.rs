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

/// Fresh food tipped into a basket that has been going over comes down to
/// meet it. Mould spreads.
#[test]
fn fresh_food_tipped_onto_old_comes_down_to_meet_it() {
    let mut older = a_lot_of("food", 10, 0);
    let this_morning = a_lot_of("food", 10, 4_000);

    older.absorb(this_morning);

    assert_eq!(older.quantity, 20, "it is all one basket now");

    let clock = older.food_data.as_ref().unwrap().created_tick;
    assert!(clock < 4_000, "the new food does not keep its own timer: {clock}");
    assert!(
        clock > 0,
        "and the basket is not pinned at the age of its very first berry: {clock}"
    );
}

/// And it cuts both ways: an older handful tipped onto a fresh stack drags the
/// stack down rather than being made new by it. Nobody would think to ask for
/// that half, and it is the same bug seen from the other side.
#[test]
fn old_food_tipped_onto_fresh_does_not_come_up_to_meet_it() {
    let mut this_morning = a_lot_of("food", 10, 4_000);
    this_morning.absorb(a_lot_of("food", 10, 0));

    let clock = this_morning.food_data.as_ref().unwrap().created_tick;
    assert!(clock < 4_000, "a stale handful tells on the basket: {clock}");
}

/// Once mould has actually manifested it takes the whole basket. Nothing
/// rescues a basket that has gone over by putting good fruit into it.
#[test]
fn good_fruit_does_not_rescue_a_basket_that_has_gone_over() {
    let mut gone_over = a_lot_of("food", 2, 0);
    if let Some(ref mut clock) = gone_over.food_data {
        clock.freshness = 0.0;
    }

    gone_over.absorb(a_lot_of("food", 100, 9_000));

    assert_eq!(
        gone_over.food_data.as_ref().unwrap().created_tick,
        0,
        "a hundred fresh berries do not save two mouldy ones - they join them"
    );
}

/// A stack keeps no better than its worst part. A dried strip dropped into a
/// raw stack does not make the raw stack keep.
#[test]
fn a_stack_keeps_no_better_than_its_worst_part() {
    let mut dried = a_lot_of("food", 10, 0);
    if let Some(ref mut clock) = dried.food_data {
        clock.preparation = PreparationState::Dried;
    }

    dried.absorb(a_lot_of("food", 10, 0));

    assert_eq!(
        dried.food_data.as_ref().unwrap().preparation,
        PreparationState::Raw,
        "a basket with raw food in it is a raw basket"
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
    assert!(
        stack.food_data.as_ref().unwrap().created_tick < 5_000,
        "a pack does not forget what it was already carrying"
    );
}

/// A pit is not a pack, and that is the whole difference. A pack has one slot
/// per name and has to merge; a pit is a list and does not. What a person does
/// with a store is put this autumn's load in **beside** last autumn's.
#[test]
fn a_pit_puts_this_load_beside_the_last_one() {
    let mut pit = Pit {
        where_it_is: Position::new(0, 0),
        holds: vec![a_lot_of("food", 40, 0)],
        covered: true,
        dug: 0,
    };

    pit.put_in(a_lot_of("food", 10, 5_000));

    assert_eq!(pit.how_much_is_in_it(), 50, "it is all in the hole");
    assert_eq!(
        pit.holds.len(),
        2,
        "and a season apart is two loads, not one"
    );
    assert_eq!(
        pit.holds[0].food_data.as_ref().unwrap().created_tick,
        0,
        "last autumn's load is untouched by this autumn's"
    );
}

/// Loads put by in the same week are the same load.
#[test]
fn a_pit_joins_up_what_went_in_together() {
    let mut pit = Pit {
        where_it_is: Position::new(0, 0),
        holds: vec![a_lot_of("food", 40, 1_000)],
        covered: true,
        dug: 0,
    };

    pit.put_in(a_lot_of("food", 10, 1_001));

    assert_eq!(pit.holds.len(), 1, "a day apart is one load");
    assert_eq!(pit.how_much_is_in_it(), 50);
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
