// src/analytics/tests/cooking_tests.rs
//! Tests for cooking: what a fire is good for, and what it destroys.
//!
//! Until this was wired up nothing in a run ever lit a fire, so the strongest
//! smell in the model and the whole preparation system sat unused. These
//! cover:
//! - only food a fire improves counts as worth cooking
//! - putting anything else over the flames ruins it, and ruined food is
//!   worthless and unsafe to eat
//! - agents gather wood, light fires and cook at them without being told to
//! - an agent carrying nothing a fire would help never bothers

use crate::agents::{AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::world::nutrition::{CookingOutcome, FoodDatabase, PreparationState};
use crate::world::{ItemType, World, WorldConfig};

/// Build a food item of the given kind, ready to carry.
fn food_item(item_id: &str, item_type: ItemType, quantity: u32) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut item = InventoryItem::new_with_weight(item_id.to_string(), quantity, 0.5);
    item.food_data = database.create_food_data(&item_type, 0);
    item
}

/// A fire helps raw flesh and grain, and nothing else.
#[test]
fn only_some_food_is_worth_cooking() {
    for improved in [ItemType::Meat, ItemType::Fish, ItemType::Grain] {
        assert_eq!(
            improved.cooking_outcome(),
            CookingOutcome::Improves,
            "{improved:?} should be worth cooking"
        );
    }

    for ruined in [
        ItemType::Food,
        ItemType::Milk,
        ItemType::Honey,
        ItemType::Bread,
        ItemType::Cheese,
        ItemType::Ale,
    ] {
        assert_eq!(
            ruined.cooking_outcome(),
            CookingOutcome::Ruins,
            "{ruined:?} should be ruined by a fire"
        );
    }

    for not_food in [ItemType::Wood, ItemType::Stone, ItemType::IronAxe] {
        assert_eq!(
            not_food.cooking_outcome(),
            CookingOutcome::NotFood,
            "{not_food:?} is not food"
        );
    }
}

/// Cooking meat is what makes it worth eating.
#[test]
fn cooking_meat_unlocks_what_is_in_it() {
    let database = FoodDatabase::new();
    let mut meat = database
        .create_food_data(&ItemType::Meat, 0)
        .expect("meat should be in the database");

    let raw = meat.effective_nutrition().total();
    assert_eq!(meat.cook(CookingOutcome::Improves), CookingOutcome::Improves);
    assert_eq!(meat.preparation, PreparationState::Cooked);

    let cooked = meat.effective_nutrition().total();
    assert!(
        cooked > raw * 2.0,
        "cooked meat should give up far more than raw: {raw} -> {cooked}"
    );
    assert!(!meat.is_harmful());
}

/// A handful of berries put over a fire is a handful of berries wasted.
#[test]
fn cooking_a_berry_ruins_it() {
    let database = FoodDatabase::new();
    let mut berries = database
        .create_food_data(&ItemType::Food, 0)
        .expect("generic food should be in the database");

    assert!(berries.effective_nutrition().total() > 0.0);

    assert_eq!(berries.cook(CookingOutcome::Ruins), CookingOutcome::Ruins);

    assert_eq!(berries.preparation, PreparationState::Ruined);
    assert!(berries.is_ruined());
    assert!(berries.is_harmful(), "ruined food should not be safe to eat");
    assert_eq!(
        berries.effective_nutrition().total(),
        0.0,
        "there should be nothing left in it"
    );
}

/// Even the right food is ruined by a second turn over the flames.
#[test]
fn cooking_something_twice_burns_it() {
    let database = FoodDatabase::new();
    let mut fish = database
        .create_food_data(&ItemType::Fish, 0)
        .expect("fish should be in the database");

    fish.cook(CookingOutcome::Improves);
    assert_eq!(fish.preparation, PreparationState::Cooked);

    assert_eq!(fish.cook(CookingOutcome::Improves), CookingOutcome::Ruins);
    assert_eq!(fish.preparation, PreparationState::Ruined);
}

/// Food an agent has ruined is food it will not eat.
#[test]
fn an_agent_will_not_eat_what_it_has_ruined() {
    let mut agent = crate::agents::Agent::new(AgentConfig::default());

    let mut burnt = food_item("burnt_meat", ItemType::Meat, 5);
    burnt
        .food_data
        .as_mut()
        .expect("meat should carry food data")
        .cook(CookingOutcome::Ruins);
    agent.inventory.add_item(burnt);

    assert!(
        !agent.has_edible_food(),
        "burnt meat is not something to eat"
    );
    assert_eq!(agent.find_best_food_to_eat(), None);
}

/// An agent with wood and raw fish gets a fire going and cooks at it.
///
/// The agent is given a practised hand so the outcome does not turn on a
/// twenty-percent chance of burning the first batch.
#[test]
fn an_agent_lights_a_fire_and_cooks_on_it() {
    let mut world = World::new(WorldConfig::default());

    // Nothing to hunt and nothing to be hunted by: this is about the fire
    world.animals.get_all_mut().clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 40, 2.0));
        agent.inventory.add_item(food_item("fish", ItemType::Fish, 20));
        agent.skills.set_skill_level(SkillType::Cooking, 8);
    }

    let mut ever_lit = false;
    for _ in 0..400 {
        simulation.tick();
        if simulation
            .world
            .heat_sources
            .all()
            .iter()
            .any(|fire| fire.is_lit)
        {
            ever_lit = true;
        }
    }

    assert!(ever_lit, "the agent should have got a fire going");

    // What it *did*, not what it is still holding. Asserting on leftovers
    // read as the same thing and is not: an agent that cooks and then eats
    // its dinner has cooked, and whether any is left at tick four hundred
    // turns on how hungry it happened to be.
    let put_on_the_fire = simulation
        .actions_taken
        .get("Cook")
        .copied()
        .unwrap_or(0);
    let came_off_badly = simulation
        .actions_failed
        .get("Cook")
        .copied()
        .unwrap_or(0);

    assert!(
        put_on_the_fire > came_off_badly,
        "the agent should have cooked something: it put food on the fire {} \
         times and {} of those came to nothing",
        put_on_the_fire,
        came_off_badly
    );
}

/// Nobody lights a fire to cook berries.
#[test]
fn an_agent_with_nothing_worth_cooking_lights_no_fire() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 40, 2.0));
        agent.inventory.add_item(food_item("food", ItemType::Food, 20));
    }

    let position = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .cooking_action(&simulation.population.agents[0], position)
            .is_none(),
        "an agent carrying only berries has no reason to light a fire"
    );

    // A whole fish is still not: it has to be cut up before it will go over
    // a fire at all - see `nutrition::Piece`.
    simulation.population.agents[0]
        .inventory
        .add_item(food_item("fish", ItemType::Fish, 5));

    assert!(
        simulation
            .cooking_action(&simulation.population.agents[0], position)
            .is_none(),
        "a whole fish chars outside and stays raw inside, which is not cooking"
    );

    // Cut into joints it changes its mind
    simulation.population.agents[0]
        .inventory
        .add_item(food_item("fishportions", ItemType::Fish, 5));

    assert!(
        simulation
            .cooking_action(&simulation.population.agents[0], position)
            .is_some(),
        "a joint of fish is worth a fire"
    );
}

/// Burnt food gives its carrier away, as decay rather than as dinner.
#[test]
fn ruined_food_smells_of_decay() {
    use crate::agents::senses::ScentType;

    let mut world = World::new(WorldConfig::default());
    world.resources.clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[1].state.position = (30, 34, 0);

    let mut burnt = food_item("burnt_fish", ItemType::Fish, 5);
    burnt
        .food_data
        .as_mut()
        .expect("fish should carry food data")
        .cook(CookingOutcome::Ruins);
    simulation.population.agents[0].inventory.add_item(burnt);

    simulation.emit_scents();

    let neighbour = &simulation.population.agents[1];
    assert!(
        neighbour
            .senses
            .smell
            .detected_scents
            .iter()
            .any(|scent| scent.scent_type == ScentType::Decay),
        "burnt food should smell, and smell wrong"
    );
    assert!(
        !neighbour
            .senses
            .smell
            .detected_scents
            .iter()
            .any(|scent| scent.scent_type == ScentType::Food),
        "nothing should be drawn to burnt food as if it were a meal"
    );
}

/// Cooked and burnt food is still the food it was made from.
#[test]
fn a_cooked_fish_is_still_a_fish() {
    use crate::agents::storage_integration::id_to_item_type;

    assert_eq!(id_to_item_type("cooked_fish"), Some(ItemType::Fish));
    assert_eq!(id_to_item_type("burnt_meat"), Some(ItemType::Meat));
    assert_eq!(id_to_item_type("fish"), Some(ItemType::Fish));
}
