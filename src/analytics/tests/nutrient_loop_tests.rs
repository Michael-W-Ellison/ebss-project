// src/analytics/tests/nutrient_loop_tests.rs
//! Tests for matter that comes back, as far as it still does.
//!
//! This file was the nutrient loop: a meal eaten or spoiled, a body buried,
//! all of it going into the ground as litter and rotting back into what the
//! next crop drew on, three fifths of it at a time. The ground does not work
//! that way now - wild ground is what it was made, and a field is worn by
//! crops and built by beans, muck and rest - see ISSUES_FOUND #246.
//!
//! What is left is the part that was never about fertility: what a body takes
//! in mostly comes out again, and what it leaves is a midden - a smell, and
//! seed - on the ground it stands on. `the_loop_turns_and_loses` and
//! `the_farmed_ground_holds_up_longer` were about the pool and have gone with
//! it; what a field does is held in `soil_ladder_tests`.

use crate::agents::{Agent, AgentConfig, InventoryItem, LifeStage, Population};
use crate::analytics::Simulation;
use crate::world::nutrition::FoodDatabase;
use crate::world::soil::Soil;
use crate::world::{ItemType, Position, World, WorldConfig};

/// Eating leaves something behind.
#[test]
fn a_meal_leaves_something_to_come_out() {
    let mut agent = Agent::new(AgentConfig::default());
    assert_eq!(agent.state.waste_carried, 0.0);

    agent.state.eat(100, 20.0);

    assert!(
        agent.state.waste_carried > 0.0,
        "a body keeps some of what it eats and passes the rest"
    );

    // And voiding it hands it over exactly once
    let voided = agent.state.void_waste();
    assert!(voided > 0.0);
    assert_eq!(agent.state.waste_carried, 0.0);
    assert_eq!(agent.state.void_waste(), 0.0);
}

/// Food that spoils in a pack falls to the ground instead of vanishing.
#[test]
fn what_spoils_in_a_pack_is_not_deleted() {
    let mut agent = Agent::new(AgentConfig::default());

    let database = FoodDatabase::new();
    let mut gone_off = InventoryItem::new_with_weight("food".to_string(), 6, 0.5);
    let food_data = database
        .create_food_data(&ItemType::Food, 0)
        .expect("generic food should be in the database");
    gone_off.food_data = Some(food_data);
    agent.inventory.add_item(gone_off);

    // Long enough that it has genuinely gone off rather than being told it has
    agent.turn_food_spoilage(100_000);

    assert!(
        agent.inventory.get_item("food").is_none(),
        "the spoiled stack should have left the pack"
    );
    assert!(
        agent.state.waste_carried >= Soil::WASTE_PER_SPOILED * 6.0 - 1e-6,
        "and all six units of it should be waiting to go on the ground: {}",
        agent.state.waste_carried
    );
}

/// Nothing took a share of food that spoiled, so more of it comes back than
/// from food somebody ate.
#[test]
fn spoiled_food_returns_more_than_eaten_food() {
    assert!(Soil::WASTE_PER_SPOILED > Soil::WASTE_PER_MEAL);
}

/// A body comes back to the ground, and a small one is worth less than a
/// grown one.
#[test]
fn a_body_comes_back_to_the_ground() {
    let (adult_soft, adult_bone) = LifeStage::Adult.body_left_behind();
    let (infant_soft, infant_bone) = LifeStage::Infant.body_left_behind();

    assert!(adult_soft > 0.0 && adult_bone > 0.0);
    assert!(
        adult_soft > infant_soft && adult_bone > infant_bone,
        "a grown body is worth more to the ground than a small one"
    );
    assert!(
        adult_soft > adult_bone,
        "there is more flesh on a body than bone"
    );
}

/// In a running simulation, what agents eat reaches the ground under them -
/// as a midden: a smell, and the seed that was in it.
#[test]
fn what_a_settlement_eats_reaches_the_ground_it_stands_on() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..10 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    // Everybody stands on one tile, so there is one place to look
    let here = Position::new(25, 25);
    for agent in &mut simulation.population.agents {
        agent.state.position = (25, 25, 0);
    }

    let before = simulation
        .world
        .grid
        .get_tile(&here)
        .map(|tile| tile.soil.has_somebody_left_something_here())
        .unwrap_or(false);
    assert!(!before, "the fixture's ground should start clean");

    for _ in 0..1500 {
        // Keep them there: this is about where what they leave ends up
        for agent in &mut simulation.population.agents {
            agent.state.position = (25, 25, 0);
        }
        simulation.take_a_turn();
    }

    let after = simulation
        .world
        .grid
        .get_tile(&here)
        .map(|tile| tile.soil.has_somebody_left_something_here())
        .unwrap_or(false);

    assert!(
        after,
        "a tile ten people lived on for fifteen hundred turns should have \
         something of theirs on it"
    );
}
