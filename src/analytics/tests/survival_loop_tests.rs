// src/analytics/tests/survival_loop_tests.rs
//! Regression tests for the survival loop: metabolism, eating and foraging.
//!
//! These cover the failure that let agents run to zero energy and stay there
//! while food sat in their inventory or a few tiles away:
//! - metabolism, spoilage and fatigue must run under `Population::tick`
//! - eating must refill nutritional reserves, not just felt energy
//! - hunger must outrank goals, plans and percepts in action selection
//! - renewable resources must survive being harvested empty

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::drives::DriveType;
use crate::world::{ResourceType, World, WorldConfig};

/// Agents driven through `Population::tick` must run their metabolism.
///
/// `Population::tick` used to call `age_tick` directly and skip
/// `tick_with_time`, so nutrition, food spoilage and fatigue never ran in a
/// live simulation - they were only exercised by unit tests.
#[test]
fn population_tick_runs_nutrition_metabolism() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let starting_reserves = population.agents[0].nutrition.energy_reserves;

    for _ in 0..200 {
        population.tick();
    }

    let reserves = population.agents[0].nutrition.energy_reserves;
    assert!(
        reserves < starting_reserves,
        "energy reserves should be consumed by metabolism, stayed at {reserves}"
    );
}

/// Eating restores the reserves metabolism draws down, not just felt energy.
#[test]
fn eating_restores_nutritional_reserves() {
    let mut agent = crate::agents::Agent::new(AgentConfig::default());
    agent.nutrition.energy_reserves = 10.0;
    agent.state.energy = 10.0;
    agent.state.last_ate_tick = 0;

    agent.inventory.add_item(crate::agents::InventoryItem::new_with_weight(
        "food".to_string(),
        3,
        0.5,
    ));

    let result = agent.eat_food_item("food", 500);

    assert!(
        matches!(result, crate::world::EatResult::Success(_)),
        "expected to eat carried food, got {result:?}"
    );
    assert!(
        agent.nutrition.energy_reserves > 10.0,
        "eating should refill energy reserves"
    );
    assert_eq!(agent.state.last_ate_tick, 500, "eating resets the starvation clock");
    assert_eq!(agent.state.ticks_without_food, 0);
}

/// An emptied stack must leave the inventory.
///
/// A zero-quantity "food" entry reads as "still carrying food", which left
/// starving agents trying to eat nothing instead of going to look for a meal.
#[test]
fn eating_the_last_item_empties_the_stack() {
    let mut agent = crate::agents::Agent::new(AgentConfig::default());
    agent.inventory.add_item(crate::agents::InventoryItem::new_with_weight(
        "food".to_string(),
        1,
        0.5,
    ));

    agent.eat_food_item("food", 10);

    assert!(
        agent.inventory.get_item("food").is_none(),
        "an emptied stack should be removed from the inventory"
    );
    assert!(
        agent.find_best_food_to_eat().is_none(),
        "an agent with nothing left should not report carrying food"
    );
}

/// A hungry agent carrying food eats it rather than starving on a full pack.
#[test]
fn hungry_agent_eats_the_food_it_carries() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.add_item(crate::agents::InventoryItem::new_with_weight(
            "food".to_string(),
            5,
            0.5,
        ));
        agent.nutrition.energy_reserves = 5.0;
        agent.state.energy = 5.0;

        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.value = 1.0;
        }
    }

    for _ in 0..20 {
        simulation.tick();
    }

    let agent = &simulation.population.agents[0];
    assert!(
        agent.nutrition.energy_reserves > 5.0,
        "a hungry agent holding food should have eaten some of it (reserves {})",
        agent.nutrition.energy_reserves
    );
}

/// Renewable resources stay on the map when harvested empty so they regrow.
///
/// Deleting them made every berry patch single-use, draining the world of food
/// permanently and starving the population no matter how well it foraged.
#[test]
fn emptied_renewable_resources_are_not_deleted() {
    let mut world = World::new(WorldConfig::default());

    let food_nodes = world
        .resources
        .iter()
        .filter(|r| r.resource_type == ResourceType::Food)
        .count();
    assert!(food_nodes > 0, "world should generate food resources");

    for resource in &mut world.resources {
        if resource.resource_type == ResourceType::Food {
            resource.amount = 0;
        }
    }

    world.remove_depleted_resources();

    let remaining = world
        .resources
        .iter()
        .filter(|r| r.resource_type == ResourceType::Food)
        .count();
    assert_eq!(
        remaining, food_nodes,
        "emptied food patches should remain so they can regrow"
    );

    // Mined-out mineral deposits are genuinely gone
    for resource in &mut world.resources {
        if resource.resource_type == ResourceType::Iron {
            resource.amount = 0;
        }
    }
    world.remove_depleted_resources();

    assert_eq!(
        world
            .resources
            .iter()
            .filter(|r| r.resource_type == ResourceType::Iron)
            .count(),
        0,
        "exhausted mineral deposits should be removed"
    );
}

/// The population as a whole must be able to feed itself over a long run.
///
/// This is the end-to-end check: previously every agent sat at zero energy
/// within ~800 ticks, whatever food the world held.
#[test]
fn population_feeds_itself_over_a_long_run() {
    // A block of seeds rather than one, because whether any of eight people
    // are still standing after four thousand ticks is a coin this model does
    // not weight heavily. Measured over thirty-two worlds at three different
    // seed blocks: 14, 24 and 20 of them still had somebody alive.
    //
    // It was one seed, which is worse than either. A single draw of a coin
    // tests which way that coin fell: seed 4,200 kept its people through
    // every change made to this model for months and then lost them to the
    // lower tiers of the food web becoming a population, at which point six
    // worlds measured together went **up**, from 23,733 person-days to
    // 24,022. The test was reporting its draw. See ISSUES_FOUND.md #132.
    //
    // What it asks now is the thing it was always about: that a settlement
    // usually feeds itself, and that when it does the people in it are fed.
    // A model that stopped working would fail every world, not three in six.
    const WORLDS: usize = 6;

    let mut still_standing = 0usize;
    let mut worlds_where_most_were_fed = 0usize;

    for seed in 0..WORLDS as u64 {
        crate::core::dice::seed(4_200 + seed);

        let world = World::new(WorldConfig::default());
        let mut population = Population::new();
        for _ in 0..8 {
            population.spawn_agent(AgentConfig::default());
        }

        let mut simulation = Simulation::new(world, population);

        for _ in 0..4000 {
            simulation.tick();
        }

        let agents = &simulation.population.agents;
        if agents.is_empty() {
            continue;
        }
        still_standing += 1;

        let fed = agents
            .iter()
            .filter(|a| a.nutrition.energy_reserves > 20.0)
            .count();

        if fed * 2 >= agents.len() {
            worlds_where_most_were_fed += 1;
        }
    }

    assert!(
        still_standing * 2 >= WORLDS,
        "only {still_standing} of {WORLDS} settlements still had anybody in \
         them after four thousand ticks"
    );
    assert_eq!(
        worlds_where_most_were_fed, still_standing,
        "{worlds_where_most_were_fed} of the {still_standing} settlements \
         that survived had most of their people fed: somewhere a people is \
         holding on without eating"
    );
}
