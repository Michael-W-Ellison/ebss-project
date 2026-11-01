// src/analytics/tests/lining_tests.rs
//! Lining a store (ISSUES_FOUND #392).
//!
//! "Woven bins", stage 1 of the storage path.
//!
//! - somebody who does not know to keeps their only basket; somebody who
//!   knows puts it in the store
//! - with no vessel, somebody who knows weaves a basket of two lashings for
//!   the store there and then
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

/// Nobody gives up their only basket for a store unless they know to.
#[test]
fn only_somebody_who_knows_puts_in_their_only_basket() {
    let mut simulation = at_a_bare_store();
    simulation.population.agents[0].inventory.add_item(InventoryItem::new_with_weight("basket".to_string(), 1, 1.0));
    simulation.line_the_store_here(0, Position::new(STORE.0, STORE.1));
    assert!(!lined(&simulation), "the only basket stays in the pack");

    simulation.population.agents[0].found_out_how_to("know:line_the_store");
    simulation.line_the_store_here(0, Position::new(STORE.0, STORE.1));
    assert!(lined(&simulation));
    assert_eq!(simulation.population.agents[0].how_many_i_have("basket"), 0);
    assert_eq!(simulation.world.stores_lined, 1);
}

/// With no vessel, a basket woven of two lashings goes in.
#[test]
fn a_basket_is_woven_for_the_store() {
    let mut simulation = at_a_bare_store();
    simulation.population.agents[0].found_out_how_to("know:line_the_store");
    simulation.population.agents[0].inventory.add_item(InventoryItem::new_with_weight("lashing".to_string(), 2, 0.2));
    simulation.line_the_store_here(0, Position::new(STORE.0, STORE.1));
    assert!(lined(&simulation));
    assert_eq!(simulation.population.agents[0].how_many_i_have("lashing"), 0);
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
