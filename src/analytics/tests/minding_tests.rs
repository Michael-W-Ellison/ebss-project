// src/analytics/tests/minding_tests.rs
//! Tests for when a parent goes after a child that has wandered (#303).
//!
//! A child of six or more who has wandered past the leash draws its parent
//! after it. Two things now hold that back: a child with somebody grown beside
//! it is minded, and a hungry parent with supper in hand eats first.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::nutrition::FoodDatabase;
use crate::world::{ItemType, World, WorldConfig};

/// A parent, their seven-year-old twenty paces off, and - if `aunt` - another
/// grown person standing beside the child.
fn a_child_wandered_off(aunt: bool) -> Simulation {
    let mut population = Population::new();
    for _ in 0..3 {
        population.spawn_agent(AgentConfig::default());
    }
    let parent = population.agents[0].id;
    let here = population.agents[0].state.position;
    let there = (here.0 + 20, here.1, here.2);

    population.agents[1].parent_ids.push(parent);
    population.agents[1].state.now_this_many_years_old(7);
    population.agents[1].state.position = there;

    population.agents[2].state.position = if aunt { there } else { (here.0 - 40, here.1, here.2) };

    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    Simulation::new(world, population)
}

fn what_the_parent_does(simulation: &Simulation) -> Option<Action> {
    let parent = simulation.population.agents[0].clone();
    simulation.protective_action(&parent, parent.state.position)
}

/// Alone out there, the child draws the parent after it.
#[test]
fn a_child_alone_past_the_leash_is_fetched() {
    let simulation = a_child_wandered_off(false);
    let child_at = simulation.population.agents[1].state.position;
    match what_the_parent_does(&simulation) {
        Some(Action::Move { target }) => assert_eq!((target.0, target.1), (child_at.0, child_at.1)),
        other => panic!("a child alone past the leash should be fetched, got {other:?}"),
    }
}

/// With somebody grown beside it, the child is minded and the parent gets on.
#[test]
fn a_child_beside_somebody_grown_is_minded() {
    let simulation = a_child_wandered_off(true);
    assert!(
        what_the_parent_does(&simulation).is_none(),
        "a child standing beside another grown person is minded"
    );
}

/// A hungry parent with supper in the pack eats before going after a child
/// that has only wandered.
#[test]
fn a_hungry_parent_eats_before_fetching_a_wandered_child() {
    let mut simulation = a_child_wandered_off(false);
    {
        let parent = &mut simulation.population.agents[0];
        let mut legumes = InventoryItem::new_with_weight("legumes".to_string(), 5, 0.2);
        legumes.food_data = FoodDatabase::new().create_food_data(&ItemType::Legumes, 0);
        parent.inventory.add_item(legumes);
        if let Some(hunger) = parent.drives.get_mut(crate::core::DriveType::Hunger) {
            hunger.value = 1.0;
        }
    }
    assert!(simulation.population.agents[0].has_edible_food());
    assert!(
        what_the_parent_does(&simulation).is_none(),
        "a hungry parent with food in hand should eat first"
    );
}
