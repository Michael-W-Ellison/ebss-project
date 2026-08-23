// src/analytics/tests/nutrient_loop_tests.rs
//! Tests for matter that comes back.
//!
//! Everything a settlement grew used to leave the world for good. Food eaten
//! was gone; food that spoiled in a pack was deleted outright, making a pack
//! the one place in the world where matter could rot to nothing; and a body
//! was buried nowhere. The soil was a stock being mined with no return at all,
//! and the only thing that ever put anything back was an agent who had learned
//! to tip a spoiled basket onto a field. Traced over thirty thousand ticks,
//! farmed ground went from 0.53 fertility to 0.03 and stayed there.
//!
//! What a body takes in mostly comes out again, and what a body is comes back
//! when it stops. Neither is free: rot keeps three fifths of what it works on
//! and loses the rest, so the loop turns and loses on every turn.

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

/// The loop loses on every turn: what comes back is less than what went in.
#[test]
fn the_loop_turns_and_loses() {
    let taken = Soil::NUTRIENT_PER_UNIT_GROWN;
    let returned = Soil::WASTE_PER_MEAL * Soil::KEPT_FROM_ROT;

    assert!(
        returned < taken,
        "a closed loop would make farming free: {returned:.5} back against {taken:.5} taken"
    );
    assert!(
        returned > taken * 0.4,
        "and it should be worth having: only {:.0}% comes back",
        returned / taken * 100.0
    );
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
    agent.tick_food_spoilage(100_000);

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

/// In a running simulation, what agents eat reaches the ground under them.
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
        .map(|tile| tile.soil.litter())
        .unwrap_or(0.0);

    for _ in 0..1500 {
        // Keep them there: this is about where what they leave ends up
        for agent in &mut simulation.population.agents {
            agent.state.position = (25, 25, 0);
        }
        simulation.tick();
    }

    let after = simulation
        .world
        .grid
        .get_tile(&here)
        .map(|tile| tile.soil.litter())
        .unwrap_or(0.0);

    assert!(
        after > before,
        "a tile ten people lived on for fifteen hundred ticks should have gained \
         litter, not lost it: {before:.3} -> {after:.3}"
    );
}

/// The ground a settlement farms holds up far better than it used to.
#[test]
fn the_farmed_ground_holds_up_longer() {
    use crate::world::ResourceType;

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    let farmed_fertility = |simulation: &Simulation| -> f32 {
        let mut total = 0.0;
        let mut patches = 0;
        for resource in &simulation.world.resources {
            if !matches!(
                resource.resource_type,
                ResourceType::Food | ResourceType::Grain
            ) {
                continue;
            }
            total += simulation
                .world
                .grid
                .get_tile(&resource.position)
                .map(|tile| tile.soil.fertility())
                .unwrap_or(0.0);
            patches += 1;
        }
        total / patches.max(1) as f32
    };

    let before = farmed_fertility(&simulation);

    for _ in 0..10_000 {
        simulation.tick();
    }

    let after = farmed_fertility(&simulation);

    // Ten thousand ticks of a settlement working the ground. Without anything
    // coming back this was already most of the way down; the loop should keep
    // it in the same country as where it started.
    assert!(
        after > before * 0.5,
        "farmed ground should not have lost half its fertility in ten thousand \
         ticks: {before:.3} -> {after:.3}"
    );
}
