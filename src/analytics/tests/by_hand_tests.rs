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

/// One who has been laid up by raw flesh twice holds raw fish portions, and
/// has a mouthful of something else in the gut.
fn wary_of_raw_flesh(days_without_enough: f32) -> Simulation {
    let mut simulation = hungry_with_fish(0, false);
    let agent = &mut simulation.population.agents[0];
    agent
        .times_laid_up
        .insert(crate::agents::Agent::OFF_RAW_FLESH.to_string(), 2);
    let mut portions = InventoryItem::new_with_weight("fishportions".to_string(), 4, 0.5);
    portions.food_data = FoodDatabase::new().create_food_data(&ItemType::Fish, 0);
    agent.inventory.add_item(portions);

    let physiology = &mut agent.state.physiology;
    let a_day = crate::agents::physiology::UNITS_BURNED_IN_AN_ORDINARY_DAY;
    physiology.reserve = (physiology.reserve_capacity - a_day * days_without_enough).max(0.0);
    // A handful of legumes a day: never an empty gut.
    physiology.eat(crate::agents::physiology::UNITS_IN_ONE_ITEM, 1.0);
    simulation
}

/// Somebody who has been made ill by raw flesh will not eat it while they
/// have a day or two of reserve gone - but three days into the reserve they
/// will, whatever else is in the gut (#300).
#[test]
fn three_days_short_and_raw_fish_is_eaten() {
    let simulation = wary_of_raw_flesh(1.0);
    let agent = &simulation.population.agents[0];
    assert!(agent.state.physiology.days_into_the_reserve() < 3.0);
    assert!(!agent.state.is_starving(), "a mouthful in the gut is not starving");
    assert_eq!(agent.find_best_food_to_eat(), None, "a day short, the raw fish is refused");

    let simulation = wary_of_raw_flesh(4.0);
    let agent = &simulation.population.agents[0];
    assert!(agent.state.physiology.days_into_the_reserve() >= 3.0);
    assert!(!agent.state.is_starving(), "still a mouthful in the gut");
    assert_eq!(
        agent.find_best_food_to_eat().as_deref(),
        Some("fishportions"),
        "four days into the reserve, the raw fish is eaten"
    );
}

/// A cooked joint is still a joint, and still supper (#300).
#[test]
fn what_comes_off_the_fire_can_be_eaten() {
    use crate::world::nutrition::Piece;
    for cut in ["meatportions", "fishportions", "fishstrips"] {
        let cooked = Simulation::prepared_item_id(cut, true);
        assert_eq!(cooked, format!("cooked_{cut}"));
        assert!(Piece::of(&cooked).can_it_be_eaten(), "{cooked} should be supper");
        assert_eq!(
            crate::agents::storage_integration::id_to_item_type(&cooked),
            crate::agents::storage_integration::id_to_item_type(cut),
            "and still the same kind of food"
        );
    }
    assert_eq!(Simulation::prepared_item_id("cooked_fishportions", false), "burnt_fishportions");
}
