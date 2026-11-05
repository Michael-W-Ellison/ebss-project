// src/analytics/tests/a_basket_kept_tests.rs
//! The carrier is kit (#412).
//!
//! One of the two who still starved after #411 set his basket down to see
//! what would become of it - curiosity, trying "put down" on the thing in his
//! pack he had tried it on least - and walked on. Bare hands hold a dozen;
//! his bow, net, rod, axe, knife and stick were more than that, so there was
//! no room left for a handful of anything, and nothing in the model made a
//! basket again for somebody who had made enough of them to have seen one.
//! He starved in twenty-five days with two cords in his pack.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};

/// One grown person on bare ground with an empty pack.
fn somebody_grown() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let agent = &mut simulation.population.agents[0];
    agent.inventory.get_all_items_mut().clear();
    agent.state.age = 25 * crate::environment::seasons::TICKS_PER_YEAR;
    assert!(agent.state.years_old() >= crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT);
    simulation
}

fn give(simulation: &mut Simulation, what: &str, how_many: u32) {
    let mut item = InventoryItem::new(what.to_string(), how_many);
    item.current_durability = Some(40.0);
    item.max_durability = Some(40.0);
    simulation.population.agents[0].inventory.add_item(item);
}

/// Curiosity does not try putting down, or cutting, the one basket - and does
/// with a second.
#[test]
fn curiosity_does_not_put_down_the_only_basket() {
    let mut simulation = somebody_grown();
    give(&mut simulation, "basket", 1);
    give(&mut simulation, "stoneknife", 1);
    let agent = &simulation.population.agents[0];
    let tries = simulation.what_i_could_try_here(agent);
    let offered = |key: &str| tries.iter().any(|(_, tried)| tried == key);
    assert!(!offered("place down:basket"), "the only basket is not put down to see");
    assert!(!offered("cut:basket"), "nor cut up to see");
    assert!(offered("examine:basket"), "it can still be looked at");

    give(&mut simulation, "basket", 1);
    let agent = &simulation.population.agents[0];
    let tries = simulation.what_i_could_try_here(agent);
    assert!(
        tries.iter().any(|(_, tried)| tried == "place down:basket"),
        "a second basket can be"
    );
}

/// Somebody who has made a basket and has none makes another, before the cord
/// for the next edge; somebody who has one, or never made one, does not.
#[test]
fn a_basket_lost_is_made_again() {
    let mut simulation = somebody_grown();
    give(&mut simulation, "stoneknife", 1);
    give(&mut simulation, "lashing", 2);
    let agent = &mut simulation.population.agents[0];
    agent.found_out_how_to("basket");
    assert!(agent.knows_how_to_make("basket"));
    assert_eq!(agent.the_carrier_i_am_without(), None, "never having made one, it is left to curiosity");

    agent.skills.made_one("basket");
    assert_eq!(agent.the_carrier_i_am_without(), Some("basket"));
    assert_eq!(agent.what_my_kit_is_short_of(), Some("basket"), "and it is what the kit is short of");
    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.how_i_would_come_by("basket", &agent),
        Some(crate::environment::Action::Craft { item_type: "basket".to_string() }),
        "with the cords for it in the pack, he weaves it"
    );

    give(&mut simulation, "basket", 1);
    assert_eq!(simulation.population.agents[0].the_carrier_i_am_without(), None);
}
