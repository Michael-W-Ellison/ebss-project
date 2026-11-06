// src/analytics/tests/lining_tests.rs
//! Lining a store (ISSUES_FOUND #392).
//!
//! "Woven bins", stage 1 of the storage path.
//!
//! - nobody gives up the only basket they forage with
//! - with no vessel to spare, somebody who knows weaves a basket for a store
//!   worth it, of lashings they can spare
//! - a basket is sealed with clay only as it goes into a store (#393)
//! - opening a store reckons what it lost: food, days and what is gone
//! - comparing bare stores with lined ones is how lining is found out (#394)
//! - the frugal line what the lazy leave bare (#394)
//! - a bare store of the camp's is walked to and lined (#394)

use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::environment::Action;
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
        last_opened: None,
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

/// What a store loses is reckoned by somebody opening it: so much food, for
/// so many days, and so much of it gone.
#[test]
fn opening_a_store_reckons_what_it_lost() {
    let mut simulation = at_a_bare_store();
    let here = Position::new(STORE.0, STORE.1);
    simulation.world.pits[0].gnawed = 5;
    simulation.current_turn = 10 * TICKS_PER_DAY;
    simulation.what_opening_the_store_shows(0, here);
    let (food_days, lost) = simulation.population.agents[0].store_losses["bare"];
    assert_eq!(lost, 5.0);
    assert!((food_days - 105.0 * 10.0).abs() < 1e-6, "{food_days}");
    assert_eq!(simulation.world.pits[0].gnawed, 0);
    assert_eq!(simulation.world.pits[0].last_opened, Some(10 * TICKS_PER_DAY));
}

/// Somebody who has watched bare stores and lined ones lose what they lose
/// sees which keeps better; somebody who has seen only bare ones does not.
#[test]
fn comparing_bare_stores_with_lined_ones_is_how_it_is_found_out() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.what_i_saw_of_a_store("bare", 1_000.0, 2.0);
    assert!(!agent.what_the_stores_have_shown_me(), "nothing to compare it with");
    agent.what_i_saw_of_a_store("lined", 1_000.0, 1.6);
    assert!(!agent.what_the_stores_have_shown_me(), "not plainly worse");
    agent.what_i_saw_of_a_store("lined", 1_000.0, 0.0);
    assert!(agent.what_the_stores_have_shown_me());
    assert!(agent.knows_the_way("line_the_store"));
    assert!(agent.did_i_come_to_it_myself("know:line_the_store"));
}

/// Whether a store is worth lining is a matter of temperament: a frugal man
/// lines a store a lazy one leaves bare.
#[test]
fn the_frugal_line_what_the_lazy_leave() {
    use crate::core::traits::{Trait, TraitSet};
    let simulation = at_a_bare_store();
    let with = |t: Trait| {
        let mut agent = Agent::new(AgentConfig::default());
        agent.traits = TraitSet::new();
        agent.traits.add_trait(t);
        agent.found_out_how_to("know:line_the_store");
        agent
    };
    let frugal = with(Trait::Frugal);
    let lazy = with(Trait::Lazy);
    assert!(simulation.is_lining_this_worth_it(&frugal, 60));
    assert!(!simulation.is_lining_this_worth_it(&lazy, 60));
    assert!(simulation.is_lining_this_worth_it(&lazy, 400), "a big enough store even for him");
}

/// A bare store of the camp's is walked to and lined by somebody who knows
/// to and has lashings to spare.
#[test]
fn a_bare_store_of_the_camps_is_lined() {
    let mut simulation = at_a_bare_store();
    simulation.world.pits[0].holds[0].quantity = 200;
    simulation.population.agents[0].found_out_how_to("know:line_the_store");
    simulation.population.agents[0].inventory.add_item(InventoryItem::new_with_weight("lashing".to_string(), 4, 0.2));
    simulation.population.agents[0].hearth = Some(STORE);
    simulation.population.agents[0].state.position = (STORE.0 + 5, STORE.1, 0);
    let agent = simulation.population.agents[0].clone();
    assert!(matches!(simulation.a_store_to_line(&agent, agent.state.position), Some(Action::Move { .. })));
    simulation.population.agents[0].state.position = (STORE.0, STORE.1, 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_store_to_line(&agent, agent.state.position), Some(Action::LineStore));
    let result = simulation.execute_action(&Action::LineStore, 0);
    assert!(result.success, "{result:?}");
    assert!(lined(&simulation));
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_store_to_line(&agent, agent.state.position), None, "nothing left bare");
}

/// With nothing to line it with, the lashings are made of wood in hand, or
/// the wood fetched for them.
#[test]
fn with_nothing_to_line_it_the_makings_are_got() {
    let mut simulation = at_a_bare_store();
    simulation.world.pits[0].holds[0].quantity = 200;
    simulation.population.agents[0].found_out_how_to("know:line_the_store");
    simulation.population.agents[0].hearth = Some(STORE);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.a_store_to_line(&agent, agent.state.position),
        Some(Action::Gather { resource_type: "wood".to_string() })
    );
    simulation.population.agents[0].inventory.add_item(InventoryItem::new_with_weight("wood".to_string(), 1, 1.0));
    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.a_store_to_line(&agent, agent.state.position),
        Some(Action::Craft { item_type: "lashing".to_string() })
    );
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
