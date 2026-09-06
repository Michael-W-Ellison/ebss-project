// src/analytics/tests/larder_reach_tests.rs
//! A full larder five paces off beats a bush across the valley.
//!
//! Measured at the last look anybody got before they died, over thirty-two
//! worlds: the settlement's pits held **805.7 items among 6.68 mouths** - ten
//! days of food for everybody - the larder was wholly empty in under one per
//! cent of those samples, and the dying were carrying one item, eleven days
//! into a three-week reserve. They starved walking somewhere.
//!
//! The store sits behind the ordinary food branch, which is right and was
//! measured. What broke it was taking the limit off the range of the food
//! search: `food_action` could no longer return `None`, so the branch behind
//! it stopped existing. It is compared on distance now.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{Pit, Position, ResourceNode, ResourceType, World, WorldConfig};

/// A country with one bush in it, at whatever distance is asked for, and a
/// larder with ten days of food in it five paces from where the man stands.
fn one_bush_and_a_full_pit(bush_at: (i32, i32)) -> crate::analytics::Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let here = population.agents[0].state.position;

    world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(here.0 + bush_at.0, here.1 + bush_at.1),
        500,
    ));

    let mut simulation = crate::analytics::Simulation::new(world, population);

    // A pit only counts as a larder if what is in it is a meal, which means
    // a stack with a clock on it that has not gone over - see `is_it_a_meal`.
    let mut buried = InventoryItem::new_with_weight("food".to_string(), 150, 0.5);
    buried.food_data = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Food, 0);

    let mut pit = Pit {
        where_it_is: Position::new(here.0 + PACES_TO_THE_PIT, here.1),
        holds: Vec::new(),
        covered: true,
        dug: 0,
    };
    pit.put_in(buried);
    simulation.world.pits.push(pit);

    // Starving, which is what opens a store while the hedgerows are bearing.
    if let Some(hunger) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Hunger)
    {
        hunger.value = 1.0;
    }
    // `is_starving` is the body's own reckoning, and energy is the half of
    // it a fixture can set without waiting a fortnight.
    simulation.population.agents[0].state.energy = 5.0;

    simulation
}

/// Near enough that nobody would walk past it, far enough to be a walk.
const PACES_TO_THE_PIT: i32 = 5;

/// The bush is across the valley and the pit is at hand: he opens the pit.
#[test]
fn a_starving_man_walks_to_the_larder_rather_than_across_the_valley() {
    let simulation = one_bush_and_a_full_pit((30, 0));
    let agent = simulation.population.agents[0].clone();
    let here = agent.state.position;

    let what = simulation
        .food_action(&agent, here, true)
        .expect("a starving man with a full pit should be doing something");

    match what {
        Action::Move { target } => assert_eq!(
            (target.0, target.1),
            (here.0 + PACES_TO_THE_PIT, here.1),
            "he set off for the far bush with ten days of food five paces away"
        ),
        Action::PickUp { .. } => {}
        other => panic!("neither the pit nor a walk to it: {other:?}"),
    }
}

/// And the bush at his feet still beats the pit, which is the ordering that
/// was measured and is not being changed: a meal out of a hole costs two
/// turns where a berry costs one.
#[test]
fn a_bush_underfoot_still_beats_the_larder() {
    let simulation = one_bush_and_a_full_pit((0, 0));
    let agent = simulation.population.agents[0].clone();
    let here = agent.state.position;

    let what = simulation
        .food_action(&agent, here, true)
        .expect("a starving man on a berry patch should be doing something");

    assert!(
        matches!(what, Action::Eat { .. } | Action::Gather { .. }),
        "he walked to the larder with a bush under his feet: {what:?}"
    );
}
