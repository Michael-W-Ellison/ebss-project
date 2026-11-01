// src/analytics/tests/lining_tests.rs
//! Lining a store (ISSUES_FOUND #392).
//!
//! "Woven bins", stage 1 of the storage path.
//!
//! - nobody gives up the only basket they forage with
//! - with no vessel to spare, somebody who knows weaves a basket for a store
//!   worth it, of lashings they can spare
//! - a basket is sealed with clay only as it goes into a store (#393)
//! - what vermin carry off is kept on the store until somebody opens it
//! - opening a gnawed store three times is how it is found out

use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::world::{Belongs, Pit, Position, World, WorldConfig};

const STORE: (i32, i32) = (25, 25);

/// One grown person standing at a covered, unlined store with food in it.
fn at_a_bare_store() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.pits.clear();
    let mut grain = InventoryItem::new_with_weight("grain".to_string(), 100, 1.0);
    grain.food_data = crate::world::nutrition::FoodDatabase::new().create_food_data(&crate::world::ItemType::Grain, 0);
    world.pits.push(Pit {
        gnawed: 0,
        where_it_is: Position::new(STORE.0, STORE.1),
        holds: vec![grain],
        covered: true,
        dug: 0,
        belongs: Belongs::ToNobody,
    });
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(30);
    agent.state.position = (STORE.0, STORE.1, 0);
    agent.inventory.get_all_items_mut().clear();
    simulation
}

fn lined(simulation: &Simulation) -> bool {
    simulation.world.pits[0].is_lined()
}

/// Nobody gives up the only basket they forage with for a store, whether
/// they know to line one or not.
#[test]
fn nobody_gives_up_their_only_basket() {
    let mut simulation = at_a_bare_store();
    simulation.population.agents[0].inventory.add_item(InventoryItem::new_with_weight("basket".to_string(), 1, 1.0));
    simulation.line_the_store_here(0, Position::new(STORE.0, STORE.1), 0);
    assert!(!lined(&simulation));
    simulation.population.agents[0].found_out_how_to("know:line_the_store");
    simulation.line_the_store_here(0, Position::new(STORE.0, STORE.1), 0);
    assert!(!lined(&simulation), "not even somebody who knows to");
    assert_eq!(simulation.population.agents[0].how_many_i_have("basket"), 1);
}

/// With lashings to spare, somebody who knows to weaves a basket for a store
/// worth it; not with only the lashings they keep back, and not for a store
/// with next to nothing in it.
#[test]
fn a_basket_is_woven_for_a_store_worth_it() {
    let mut short = at_a_bare_store();
    short.population.agents[0].found_out_how_to("know:line_the_store");
    short.population.agents[0].inventory.add_item(InventoryItem::new_with_weight("lashing".to_string(), 2, 0.2));
    short.line_the_store_here(0, Position::new(STORE.0, STORE.1), 0);
    assert!(!lined(&short), "two lashings are kept back for tools");

    let mut simulation = at_a_bare_store();
    simulation.population.agents[0].found_out_how_to("know:line_the_store");
    simulation.population.agents[0].inventory.add_item(InventoryItem::new_with_weight("lashing".to_string(), 4, 0.2));
    simulation.line_the_store_here(0, Position::new(STORE.0, STORE.1), 0);
    assert!(lined(&simulation));
    assert_eq!(simulation.population.agents[0].how_many_i_have("lashing"), 2);
    assert_eq!(simulation.world.stores_lined, 1);

    let mut empty = at_a_bare_store();
    empty.world.pits[0].holds.clear();
    empty.population.agents[0].found_out_how_to("know:line_the_store");
    empty.population.agents[0].inventory.add_item(InventoryItem::new_with_weight("lashing".to_string(), 4, 0.2));
    empty.line_the_store_here(0, Position::new(STORE.0, STORE.1), 10);
    assert!(!lined(&empty), "not for ten units");
}

/// What vermin carry off is kept on the store, and opening it three times
/// to find it gnawed is how somebody comes to line the next one.
#[test]
fn opening_a_gnawed_store_is_how_it_is_found_out() {
    let mut simulation = at_a_bare_store();
    let here = Position::new(STORE.0, STORE.1);
    for time in 0..Agent::GNAWED_STORES_BEFORE_LINING {
        for _ in 0..20 {
            simulation.world.what_the_vermin_take();
        }
        assert!(simulation.world.pits[0].gnawed > 0, "vermin have been at it");
        assert!(!simulation.population.agents[0].knows_the_way("line_the_store"), "not yet, at {time}");
        simulation.what_opening_the_store_shows(0, here);
        assert_eq!(simulation.world.pits[0].gnawed, 0, "seen, and counted once");
    }
    assert!(simulation.population.agents[0].knows_the_way("line_the_store"));
}

/// Somebody who knows to seal a lining, with clay to hand, daubs the basket
/// shut as it goes in, and vermin take from it as little as from a fired pot
/// (#393).
#[test]
fn a_basket_with_clay_to_hand_is_sealed_and_vermin_hardly_take_from_it() {
    let mut simulation = at_a_bare_store();
    let agent = &mut simulation.population.agents[0];
    agent.found_out_how_to("know:line_the_store");
    agent.found_out_how_to("know:seal_the_store");
    agent.inventory.add_item(InventoryItem::new_with_weight("basket".to_string(), 2, 1.0));
    agent.inventory.add_item(InventoryItem::new_with_weight("clay".to_string(), 1, 1.0));
    simulation.line_the_store_here(0, Position::new(STORE.0, STORE.1), 0);
    assert!(simulation.world.pits[0].holds.iter().any(|item| item.item_id == "sealedbasket"));
    assert_eq!(simulation.population.agents[0].how_many_i_have("clay"), 0);
    assert_eq!(simulation.world.pits[0].what_vermin_get_at(), Pit::VERMIN_PAST_A_POT);
    assert_eq!(simulation.world.stores_sealed, 1);
}

/// Opening lined stores gnawed three times is how sealing comes to somebody
/// who lines them (#393).
#[test]
fn lined_stores_gnawed_is_how_sealing_is_found_out() {
    let mut simulation = at_a_bare_store();
    let here = Position::new(STORE.0, STORE.1);
    simulation.population.agents[0].found_out_how_to("know:line_the_store");
    simulation.population.agents[0].inventory.add_item(InventoryItem::new_with_weight("basket".to_string(), 2, 1.0));
    simulation.line_the_store_here(0, here, 0);
    for _ in 0..Agent::LINED_STORES_GNAWED_BEFORE_SEALING {
        assert!(!simulation.population.agents[0].knows_the_way("seal_the_store"));
        for _ in 0..40 {
            simulation.world.what_the_vermin_take();
        }
        simulation.what_opening_the_store_shows(0, here);
    }
    assert!(simulation.population.agents[0].knows_the_way("seal_the_store"));
}
