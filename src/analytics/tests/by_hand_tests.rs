// src/analytics/tests/by_hand_tests.rs
//! Tests for supper with no knife: a whole fish or joint pulled apart by hand.
//!
//! People on the big map starved at the end of winter carrying thirty fish,
//! because a whole fish wants cutting, cutting wants an edge, and by then the
//! knives had worn out (#300).

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::nutrition::FoodDatabase;
use crate::world::{ItemType, World, WorldConfig};

/// One hungry person holding nothing but `fish` whole raw fish, and a knife
/// if `knife`.
fn hungry_with_fish(fish: u32, knife: bool) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let agent = &mut simulation.population.agents[0];
    let held: Vec<(String, u32)> = agent
        .inventory
        .get_all_items()
        .iter()
        .map(|(id, item)| (id.clone(), item.quantity))
        .collect();
    for (id, quantity) in held {
        agent.inventory.remove_item(&id, quantity);
    }
    agent.inventory.max_weight = 500.0;
    let mut whole = InventoryItem::new_with_weight("fish".to_string(), fish, 0.5);
    whole.food_data = FoodDatabase::new().create_food_data(&ItemType::Fish, 0);
    agent.inventory.add_item(whole);
    if knife {
        agent.inventory.add_item(InventoryItem::new("stoneknife".to_string(), 1));
    }
    simulation
}

/// With no edge, a whole fish is pulled apart and eaten.
#[test]
fn a_whole_fish_is_eaten_without_a_knife() {
    let mut simulation = hungry_with_fish(3, false);
    assert!(
        simulation.population.agents[0].find_best_food_to_eat().is_none(),
        "a whole fish is not supper as it stands"
    );
    let before = simulation.population.agents[0].state.physiology.in_the_stomach();

    let result = simulation.execute_action(&Action::Eat { food_type: "generic".to_string() }, 0);

    let agent = &simulation.population.agents[0];
    assert!(result.success, "eating should succeed: {:?}", result.message);
    assert_eq!(agent.how_many_i_have("fish"), 2, "one fish came apart");
    assert!(
        agent.state.physiology.in_the_stomach() > before,
        "and something went down"
    );
}

/// Hands save half what a knife would, and never less than a piece.
#[test]
fn hands_save_half_of_what_a_knife_does() {
    let mut simulation = hungry_with_fish(1, false);
    let agent = &mut simulation.population.agents[0];
    let database = FoodDatabase::new();
    let pieces = agent.pull_apart_by_hand(|as_food| database.create_food_data(&as_food, 0));
    let by_knife = crate::environment::making::how_to_work("cut", "fish")
        .expect("a fish can be cut")
        .how_many;

    assert_eq!(pieces, Some((by_knife / 2).max(1)));
    assert_eq!(agent.how_many_i_have("fish"), 0);
    assert_eq!(agent.how_many_i_have("fishportions"), (by_knife / 2).max(1));
}

/// With an edge to hand the fish is cut properly, not torn.
#[test]
fn with_a_knife_the_fish_is_not_torn() {
    let mut simulation = hungry_with_fish(2, true);
    let agent = &mut simulation.population.agents[0];
    let database = FoodDatabase::new();

    assert!(agent.what_flesh_i_should_cut_up().is_some(), "the knife cuts it");
    assert_eq!(agent.pull_apart_by_hand(|as_food| database.create_food_data(&as_food, 0)), None);
    assert_eq!(agent.how_many_i_have("fish"), 2);
}
