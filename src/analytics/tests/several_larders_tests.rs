// src/analytics/tests/several_larders_tests.rs
//! Somewhere to go for food that is not the bush in front of you.
//!
//! "If an agent has seen a food pit with food inside it, they should remember
//! the location of the pit as a place which might satisfy their hunger drive.
//! ... this gives them several locations where they can go for food. There is
//! also barter, which should enable the trading of food."
//!
//! Two things were in the way. `SpatialMemoryType::Storage` had a reader and
//! **no writer at all** - nobody had ever remembered a pit - and the decision
//! covered for it by asking the world, `nearest_full_pit`, which is
//! omniscience. And `what_i_can_spare`, the whole of what barter could offer,
//! excludes food by name and by nutrition data, so **the one thing everybody
//! needs every day was the one thing barter could not move**.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::core::memory::SpatialMemoryType;
use crate::core::DriveType;
use crate::world::{Pit, Position, World, WorldConfig};

/// One person, and a pit of food a few paces off.
fn somebody_and_a_full_pit() -> crate::analytics::Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (10, 10, 0);

    let mut buried = InventoryItem::new_with_weight("food".to_string(), 60, 0.5);
    buried.food_data = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Food, 0);

    let mut pit = Pit {
        where_it_is: Position::new(13, 10),
        holds: Vec::new(),
        covered: true,
        dug: 0,
    };
    pit.put_in(buried);
    simulation.world.pits.push(pit);
    simulation
}

/// Seeing a pit with food in it is remembering it.
#[test]
fn a_pit_seen_with_food_in_it_is_remembered() {
    let mut simulation = somebody_and_a_full_pit();

    assert!(
        simulation.population.agents[0]
            .memory
            .recall_locations(SpatialMemoryType::Storage)
            .is_empty(),
        "the fixture starts with nothing remembered"
    );

    simulation.tick();

    let remembered = simulation.population.agents[0]
        .memory
        .recall_locations(SpatialMemoryType::Storage);
    assert_eq!(
        remembered.len(),
        1,
        "a larder three paces off went unremembered"
    );
    assert_eq!(remembered[0].position, (13, 10, 0));
    assert!(
        remembered[0].value > 0.0,
        "remembered the place and not that there was anything in it"
    );
}

/// And the store branch walks to the pit the agent remembers.
#[test]
fn the_store_branch_reads_the_memory() {
    let mut simulation = somebody_and_a_full_pit();
    simulation.tick();

    let agent = simulation.population.agents[0].clone();
    let (where_it_is, paces) = simulation
        .nearest_pit_i_remember(&agent, agent.state.position)
        .expect("he saw it a moment ago");

    assert_eq!((where_it_is.x, where_it_is.y), (13, 10));
    assert_eq!(paces, 3);
}

/// A pit nobody has ever seen is not a place anybody goes.
#[test]
fn a_pit_never_seen_is_not_remembered() {
    let simulation = somebody_and_a_full_pit();
    let agent = simulation.population.agents[0].clone();

    assert!(
        simulation
            .nearest_pit_i_remember(&agent, agent.state.position)
            .is_none(),
        "he walked to a larder he had never laid eyes on"
    );
}

/// Two people, one with a week's food and one with none.
fn one_fed_and_one_with_nothing() -> crate::analytics::Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);

    simulation.population.agents[0].state.position = (10, 10, 0);
    simulation.population.agents[1].state.position = (10, 10, 0);

    let mut carried = InventoryItem::new_with_weight("food".to_string(), 40, 0.5);
    carried.food_data = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Food, 0);
    simulation.population.agents[0].inventory.add_item(carried);

    if let Some(hunger) = simulation.population.agents[1]
        .drives
        .get_mut(DriveType::Hunger)
    {
        hunger.value = 1.0;
    }
    simulation
}

/// A man with a week's food hands some to the man beside him with none.
#[test]
fn a_hungry_neighbour_with_nothing_is_handed_a_meal() {
    let simulation = one_fed_and_one_with_nothing();

    let offer = simulation.what_i_would_hand_over(0, 1);
    let (what, how_many) = offer.expect("nothing offered to a man with nothing to eat");

    assert!(
        crate::world::nutrition::is_this_food(&what),
        "offered {what}, which is not food"
    );
    assert!(how_many > 0);

    // And he keeps a day's eating back for himself.
    let kept = simulation.population.agents[0].how_many_meals_i_have() - how_many;
    assert!(
        kept >= crate::analytics::Simulation::what_a_day_of_food_is(),
        "handed over so much he is short himself: kept {kept}"
    );
}

/// The decision reaches for it, rather than waiting for a sociable mood.
#[test]
fn the_decision_hands_it_over_without_waiting_to_feel_sociable() {
    let simulation = one_fed_and_one_with_nothing();
    let agent = simulation.population.agents[0].clone();

    assert_eq!(
        simulation.somebody_beside_me_with_nothing_to_eat(&agent, agent.state.position),
        Some(simulation.population.agents[1].id),
        "nobody was worth feeding"
    );
}

/// And nobody hands over what they need themselves.
#[test]
fn nobody_strips_their_own_pack() {
    let mut simulation = one_fed_and_one_with_nothing();

    // Down to a single day's food.
    let all = simulation.population.agents[0].how_many_meals_i_have();
    let keep = crate::analytics::Simulation::what_a_day_of_food_is();
    simulation.population.agents[0]
        .inventory
        .remove_item("food", all - keep);

    let agent = simulation.population.agents[0].clone();
    assert!(
        simulation
            .somebody_beside_me_with_nothing_to_eat(&agent, agent.state.position)
            .is_none(),
        "a man with one day's food gave it away"
    );
}
