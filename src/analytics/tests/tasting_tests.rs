// src/analytics/tests/tasting_tests.rs
//! Tests for finding out what is food by eating it.
//!
//! "A curious agent might taste a random plant. If the plant is edible, the
//! agent survives and thrives. If the plant is toxic or inedible, the agent
//! dies or starves."
//!
//! A world carries four sorts of plant nobody has tried. Which of them are
//! supper is drawn when the country is made and written nowhere anybody living
//! in it can read. The only way to find out is for somebody to put one in his
//! mouth, and what it costs him when he is wrong runs from a bad afternoon to
//! everything. What makes it worth doing at all is that the people standing
//! round him learn it for nothing.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{
    Position, ResourceNode, ResourceType, Terrain, TerrainType, World, WorldConfig,
};

/// A world where plant 0 is food and plant 1 is not, with one person standing
/// on a patch of `kind`
fn somebody_at_a_strange_plant(kind: u8, how_many_watching: usize) -> (Simulation, Position) {
    let where_it_is = Position::new(25, 25);

    let mut world = World::new(WorldConfig::default());

    // Nothing else edible anywhere: these tests are about whether this
    // particular plant is picked, and a world full of berry bushes answers a
    // request for food with berries every time
    world.resources.retain(|resource| {
        resource.position != where_it_is
            && !matches!(
                resource.resource_type,
                ResourceType::Food
                    | ResourceType::Grain
                    | ResourceType::Fish
                    | ResourceType::Meat
                    | ResourceType::StrangePlant
            )
    });

    if let Some(tile) = world.grid.get_tile_mut(&where_it_is) {
        tile.terrain = Terrain::new(TerrainType::Plains);
    }

    // Fixed, so the test is about the mechanism rather than the draw
    world.what_the_strange_plants_are = vec![true, false, true, false];
    world.resources.push(ResourceNode::of_kind(
        ResourceType::StrangePlant,
        where_it_is,
        30,
        kind,
    ));

    let mut population = Population::new();
    for _ in 0..(1 + how_many_watching) {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);
    for agent in &mut simulation.population.agents {
        agent.state.position = (where_it_is.x, where_it_is.y, 0);
    }

    (simulation, where_it_is)
}

// --------------------------------------------------------------------------
// What the world knows and nobody else does
// --------------------------------------------------------------------------

/// Every world has some of each, so curiosity is never simply free and never
/// simply fatal.
#[test]
fn every_world_has_both_kinds_of_strange_plant() {
    for _ in 0..20 {
        let world = World::new(WorldConfig::default());

        assert_eq!(
            world.what_the_strange_plants_are.len(),
            World::HOW_MANY_STRANGE_PLANTS as usize
        );
        assert!(
            world.what_the_strange_plants_are.iter().any(|good| *good),
            "some of them have to be food"
        );
        assert!(
            world.what_the_strange_plants_are.iter().any(|good| !*good),
            "and some of them have to not be"
        );
    }
}

/// They are actually out there in the country.
#[test]
fn strange_plants_grow_in_the_world() {
    let world = World::new(WorldConfig::default());

    let patches: Vec<_> = world
        .resources
        .iter()
        .filter(|resource| resource.resource_type == ResourceType::StrangePlant)
        .collect();

    assert!(
        patches.len() >= 4,
        "a country should carry a few of them, not {}",
        patches.len()
    );

    let kinds: std::collections::HashSet<u8> =
        patches.iter().map(|resource| resource.kind).collect();
    assert!(
        kinds.len() >= 2,
        "and more than one sort of them: {kinds:?}"
    );
}

/// Nobody is born knowing.
#[test]
fn nobody_starts_out_knowing_what_the_strange_plants_are() {
    let mut population = Population::new();
    for _ in 0..20 {
        population.spawn_agent(AgentConfig::default());
    }

    for agent in &population.agents {
        for kind in 0..World::HOW_MANY_STRANGE_PLANTS {
            assert!(
                !agent.have_i_tried_that_plant(kind),
                "a founder has never eaten one"
            );
            assert!(!agent.is_that_plant_food(kind));
        }
    }
}

// --------------------------------------------------------------------------
// Eating one
// --------------------------------------------------------------------------

/// One that is food feeds the man who tried it, and he knows it now.
#[test]
fn a_good_plant_feeds_the_man_who_tried_it() {
    let (mut simulation, _) = somebody_at_a_strange_plant(0, 0);

    let health_before = simulation.population.agents[0].state.health;

    let result = simulation.execute_action(&Action::Taste, 0);
    assert!(result.success, "plant 0 is food: {:?}", result.message);

    let agent = &simulation.population.agents[0];
    assert!(
        agent.is_that_plant_food(0),
        "and he is in no doubt about it afterwards"
    );
    assert_eq!(
        agent.state.health, health_before,
        "eating supper does nobody any harm"
    );
}

/// One that is not makes him ill, and he knows that too.
#[test]
fn a_bad_plant_makes_the_man_who_tried_it_ill() {
    let (mut simulation, _) = somebody_at_a_strange_plant(1, 0);

    let health_before = simulation.population.agents[0].state.health;

    let result = simulation.execute_action(&Action::Taste, 0);
    assert!(!result.success, "plant 1 is poison");

    let agent = &simulation.population.agents[0];
    assert!(
        agent.have_i_tried_that_plant(1),
        "he has an opinion about it now"
    );
    assert!(
        !agent.is_that_plant_food(1),
        "and the opinion is not that it is supper"
    );
    assert!(
        agent.state.health < health_before,
        "it cost him: {} against {}",
        agent.state.health,
        health_before
    );
}

/// Sometimes it costs him everything.
#[test]
fn a_bad_plant_can_kill() {
    let mut died = 0;

    for _ in 0..60 {
        let (mut simulation, _) = somebody_at_a_strange_plant(1, 0);

        // Somebody already in poor condition, which is who a bad plant
        // actually kills
        simulation.population.agents[0].state.health = 30.0;
        simulation.execute_action(&Action::Taste, 0);

        if simulation.population.agents[0].state.health <= 0.0 {
            died += 1;
        }
    }

    assert!(
        died > 0,
        "a strange plant should sometimes be the end of somebody"
    );
    assert!(
        died < 60,
        "and should not always be: {died} of 60"
    );
}

/// Everyone standing about learns it for nothing. This is the whole value of
/// being a people rather than a person.
#[test]
fn everybody_watching_learns_what_it_cost_him() {
    let (mut simulation, where_it_is) = somebody_at_a_strange_plant(1, 3);

    // One of them well out of sight
    simulation.population.agents[3].state.position = (where_it_is.x + 40, where_it_is.y, 0);

    simulation.execute_action(&Action::Taste, 0);

    for who in 0..3 {
        assert!(
            simulation.population.agents[who].have_i_tried_that_plant(1),
            "agent {who} was standing right there"
        );
        assert!(!simulation.population.agents[who].is_that_plant_food(1));
    }

    assert!(
        !simulation.population.agents[3].have_i_tried_that_plant(1),
        "the one forty tiles away heard nothing"
    );

    // And only the man who ate it paid for it
    assert!(
        simulation.population.agents[1].state.health
            > simulation.population.agents[0].state.health,
        "watching costs nothing"
    );
}

/// Nobody tries the same plant twice.
#[test]
fn nobody_tries_the_same_plant_twice() {
    let (mut simulation, _) = somebody_at_a_strange_plant(1, 0);

    let position = simulation.population.agents[0].state.position;

    {
        let agent = &mut simulation.population.agents[0];
        if let Some(drive) = agent.drives.get_mut(DriveType::Curiosity) {
            drive.value = 1.0;
        }
        agent.now_i_know_that_plant(1, false);
    }

    for _ in 0..500 {
        assert!(
            simulation
                .tasting_action(&simulation.population.agents[0], position)
                .is_none(),
            "he has eaten one of those and does not need reminding"
        );
    }
}

// --------------------------------------------------------------------------
// What a known plant is worth afterwards
// --------------------------------------------------------------------------

/// Once somebody knows, it is food like anything else.
#[test]
fn a_plant_known_to_be_food_can_be_gathered_and_eaten() {
    let (mut simulation, _) = somebody_at_a_strange_plant(0, 0);

    // Before anybody has tried it, nobody picks it
    let ignorant = simulation.execute_action(
        &Action::Gather {
            resource_type: "food".to_string(),
        },
        0,
    );
    assert!(
        !ignorant.success
            || simulation.population.agents[0].how_many_i_have("food") == 0,
        "nobody fills a basket with a plant they have never seen eaten"
    );

    simulation.population.agents[0].now_i_know_that_plant(0, true);

    let knowing = simulation.execute_action(
        &Action::Gather {
            resource_type: "food".to_string(),
        },
        0,
    );
    assert!(
        knowing.success,
        "a man who knows it is food picks it: {:?}",
        knowing.message
    );
    assert!(
        simulation.population.agents[0].how_many_i_have("food") > 0,
        "and it goes in the pack as food"
    );
}

/// And nobody picks the one that poisoned them.
#[test]
fn nobody_gathers_a_plant_they_know_is_poison() {
    let (mut simulation, _) = somebody_at_a_strange_plant(1, 0);
    simulation.population.agents[0].now_i_know_that_plant(1, false);

    simulation.execute_action(
        &Action::Gather {
            resource_type: "food".to_string(),
        },
        0,
    );

    assert_eq!(
        simulation.population.agents[0].how_many_i_have("food"),
        0,
        "knowing what it is is exactly what stops him"
    );
}

