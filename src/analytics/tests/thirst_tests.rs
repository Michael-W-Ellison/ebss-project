// src/analytics/tests/thirst_tests.rs
//! Regression tests for drinking, dehydration and how survival harm reaches
//! an agent's health.
//!
//! These cover the failure that left agents parched for their whole lives:
//! - thirst is acted on, not just tracked, so agents drink and keep drinking
//! - a carried waterskin can be drunk from away from open water
//! - dehydration and other survival harm actually reduce health, instead of
//!   being wiped by the body-condition sync every tick
//! - health recovers once the agent is fed, watered and unhurt

use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::drives::DriveType;
use crate::world::{World, WorldConfig};

/// Agents drink over a long run instead of drinking once and never again.
///
/// Thirst used to be reachable only through the drive-based fallback at the
/// bottom of action selection, which hunger monopolised: agents went thousands
/// of ticks without water with a river a dozen tiles away.
#[test]
fn agents_keep_themselves_watered() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..6 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    for _ in 0..3000 {
        simulation.tick();
    }

    let agents = &simulation.population.agents;
    assert!(!agents.is_empty(), "population should not have died out");

    let parched = agents
        .iter()
        .filter(|a| a.state.ticks_without_water > 1440)
        .count();

    assert_eq!(
        parched,
        0,
        "no agent should go a day without drinking; longest was {} ticks",
        agents
            .iter()
            .map(|a| a.state.ticks_without_water)
            .max()
            .unwrap_or(0)
    );

    let dehydrated = agents.iter().filter(|a| a.state.is_dehydrated()).count();
    assert_eq!(dehydrated, 0, "no agent should end the run dehydrated");
}

/// A waterskin is worth carrying: an agent away from open water drinks from it.
#[test]
fn agents_drink_from_a_carried_container() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    // Put the agent somewhere with a full waterskin and a raging thirst
    {
        let agent = &mut simulation.population.agents[0];

        let mut waterskin = InventoryItem::new_with_weight("waterskin".to_string(), 1, 0.5);
        waterskin.max_capacity = Some(2.0);
        waterskin.fill_level = Some(2.0);
        agent.inventory.add_item(waterskin);

        if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
            thirst.value = 1.0;
        }
        agent.state.last_drank_tick = 0;
    }

    // Remove every water source so only the container can help
    simulation
        .world
        .resources
        .retain(|r| r.resource_type != crate::world::ResourceType::Water);

    for _ in 0..40 {
        simulation.tick();
    }

    let agent = &simulation.population.agents[0];

    assert!(
        agent.state.ticks_without_water < 40,
        "an agent with a full waterskin should have drunk from it, {} ticks dry",
        agent.state.ticks_without_water
    );
}

/// Dehydration has to reach the agent's health, or the drive means nothing.
///
/// Health was overwritten from body condition every tick, so starvation,
/// dehydration and exposure damage were all silently discarded: an agent could
/// go six thousand ticks without water and still read as near perfect health.
#[test]
fn dehydration_damages_health() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.last_drank_tick = 0;
    agent.state.last_ate_tick = 0;

    // Well past the point where thirst starts doing harm
    let mut tick = 5000;
    let starting_health = agent.state.health;

    for _ in 0..200 {
        agent.tick_with_percepts(tick);
        agent.process_survival_tick(tick);
        tick += 1;
    }

    assert!(
        agent.state.health < starting_health,
        "prolonged dehydration should cost health, stayed at {}",
        agent.state.health
    );
}

/// Health recovers when nothing is wrong. `regenerate_health` had no callers,
/// so agents could only ever lose condition over a lifetime.
#[test]
fn health_recovers_when_fed_and_watered() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.health = 50.0;

    let mut tick = 100;
    for _ in 0..200 {
        // Keep the agent fed and watered so nothing is harming it.
        //
        // Through the **body**, not the turn counters. `age_tick_with_modifier`
        // says in its own comment that those counters "are kept only for the
        // interface and for older tests to read, and are derived rather than
        // counted so they cannot disagree with the body" - so setting them was
        // writing to a readout. The body dried out regardless, thirst took
        // health off faster than it could come back, and this test had been
        // asking whether health recovers while quietly dehydrating the man.
        agent.state.physiology.hydration = 1.0;
        agent.state.physiology.reserve = agent.state.physiology.reserve_capacity;
        agent.state.last_ate_tick = tick;
        agent.state.last_drank_tick = tick;

        agent.tick_with_percepts(tick);
        agent.process_survival_tick(tick);
        tick += 1;
    }

    assert!(
        agent.state.health > 50.0,
        "a healthy, fed, watered agent should recover, stayed at {}",
        agent.state.health
    );
}

/// Agents leave food alone once it has turned, rather than eating themselves
/// to death one bite a tick.
#[test]
fn agents_refuse_food_that_would_make_them_sick() {
    use crate::world::{FoodDatabase, ItemType};

    let mut agent = Agent::new(AgentConfig::default());

    let database = FoodDatabase::default();
    let mut rotten = InventoryItem::new_with_weight("food".to_string(), 5, 0.5);
    let mut food_data = database
        .create_food_data(&ItemType::Food, 0)
        .expect("generic food should be in the database");
    food_data.freshness = 0.05; // well past turning
    rotten.food_data = Some(food_data);
    agent.inventory.add_item(rotten);

    assert!(
        agent.find_best_food_to_eat().is_none(),
        "rotten food should not be chosen as the best thing to eat"
    );
    assert!(
        !agent.has_edible_food(),
        "an agent holding only rotten food is not carrying anything edible"
    );
}
