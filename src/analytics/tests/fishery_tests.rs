// src/analytics/tests/fishery_tests.rs
//! Tests for the one food a settlement does not grow.
//!
//! Every other return path in this simulation gives the ground back some part
//! of what the ground already paid out, and rot takes its cut on the way, so
//! the best a farming people can do is run down slowly. A fish is different in
//! kind. It was grown at sea and fed on a whole catchment, and it comes up the
//! river under its own power whatever last year's fishing left behind. What is
//! left of it, put on a field, makes the country richer than it was.
//!
//! Which is why people who had rivers buried fish with the seed corn for
//! thousands of years before anybody could say what nitrogen was.

use crate::agents::practices::Undertaking;
use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::Season;
use crate::environment::Action;
use crate::world::soil::Soil;
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// A patch of river, and a bank beside it.
fn a_world_with_a_river() -> World {
    let mut world = World::new(WorldConfig::default());

    for x in 0..6 {
        for y in 0..3 {
            let here = Position::new(x, y);
            if let Some(tile) = world.grid.get_tile_mut(&here) {
                tile.terrain.terrain_type = if y == 0 {
                    TerrainType::Water
                } else {
                    TerrainType::Riverbank
                };
            }
        }
    }

    world
}

/// Fish do not grow out of the bank they are caught from.
///
/// They used to. `regenerate_in_ground` drew nutrient from the tile for every
/// resource that grew on it, which had a riverbank feeding the fish in the
/// river beside it. Nothing in the world works that way round.
#[test]
fn a_fish_takes_nothing_out_of_the_bank() {
    let mut soil = Soil::for_terrain(TerrainType::Riverbank);
    let before = soil.nutrients;

    let mut reach = ResourceNode::new(ResourceType::Fish, Position::new(1, 1), 10);
    reach.max_amount = 500;

    for _ in 0..200 {
        reach.regenerate_in_ground(15.0, 0.8, 1.0, false, &mut soil);
    }

    assert_eq!(
        soil.nutrients, before,
        "a fish is grown at sea; the bank pays nothing towards it"
    );

    // And a crop on the same ground does draw on it, so the test is not
    // passing because nothing grew at all
    let mut crop = ResourceNode::new(ResourceType::Food, Position::new(1, 1), 10);
    crop.max_amount = 500;
    for _ in 0..200 {
        crop.regenerate_in_ground(15.0, 0.8, 1.0, false, &mut soil);
    }

    assert!(
        soil.nutrients < before,
        "a plant, by contrast, grows out of the ground it stands in"
    );
}

/// A reach fished down to nothing fills again from upstream.
///
/// This is the whole difference between a fishery and a berry hedge. A hedge
/// regrows out of what is left of itself, so taking all of it ends it. Fish
/// are spawned upstream and fed at sea and come back regardless.
#[test]
fn a_reach_fished_out_fills_again() {
    let mut world = a_world_with_a_river();
    world.resources.retain(|r| r.resource_type != ResourceType::Fish);

    let mut reach = ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 60);
    reach.amount = 0; // fished to nothing
    world.resources.push(reach);

    for _ in 0..4_000 {
        world.tick();
    }

    let after = world
        .resources
        .iter()
        .find(|r| r.resource_type == ResourceType::Fish)
        .map(|r| r.amount)
        .unwrap_or(0);

    assert!(
        after > 30,
        "an empty reach should be carrying fish again inside four thousand \
         ticks, not stay empty for ever; it held {after}"
    );
}

/// The run is a run: heavy twice a year and thin between.
#[test]
fn the_run_comes_in_spring_and_autumn() {
    let reach = ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 60);

    let spring = reach.fish_run(TerrainType::Water, Season::Spring, false);
    let summer = reach.fish_run(TerrainType::Water, Season::Summer, false);
    let fall = reach.fish_run(TerrainType::Water, Season::Fall, false);
    let winter = reach.fish_run(TerrainType::Water, Season::Winter, false);

    assert!(spring > fall, "the spring run is the heavier of the two");
    assert!(fall > summer, "high summer is after the run, not in it");
    assert!(summer > winter, "and winter is thinnest of all");
    assert!(winter > 0.0, "but a river is never quite empty");

    let frozen = reach.fish_run(TerrainType::Water, Season::Winter, true);
    assert!(frozen < winter, "a frozen river gives up almost nothing");

    // A river carries a run; a pond gets whatever wandered in
    let pond = reach.fish_run(TerrainType::Plains, Season::Spring, false);
    assert!(
        pond < spring,
        "the run comes up the river, not across the fields"
    );
}

/// Nothing on land is worth what a fish is worth to a field.
#[test]
fn a_fish_is_worth_many_times_a_turnip() {
    assert!(
        Soil::waste_from_eating("fish") > Soil::waste_from_eating("grain") * 10.0,
        "a crop meal returns what the ground already paid out; a fish meal is \
         something the ground never had"
    );

    assert!(
        Soil::waste_from_spoilage("fish") > Soil::waste_from_eating("fish"),
        "a fish nobody got to still has in it everything a body would have kept"
    );

    assert!(Soil::came_out_of_the_water("cooked_fish"));
    assert!(Soil::came_out_of_the_water("salmon"));
    assert!(!Soil::came_out_of_the_water("grain"));
    assert!(!Soil::came_out_of_the_water("berries"));
}

/// Eating a fish loads a body with more to pass than eating anything else.
#[test]
fn what_a_fish_leaves_reaches_the_ground() {
    let mut on_fish = Agent::new(AgentConfig::default());
    let mut on_grain = Agent::new(AgentConfig::default());

    on_fish
        .inventory
        .add_item(InventoryItem::new("fish".to_string(), 4));
    on_grain
        .inventory
        .add_item(InventoryItem::new("grain".to_string(), 4));

    let _ = on_fish.eat_food_item("fish", 100);
    let _ = on_grain.eat_food_item("grain", 100);

    assert!(
        on_fish.state.waste_carried > on_grain.state.waste_carried,
        "a meal of fish leaves more behind than a meal of grain: {} against {}",
        on_fish.state.waste_carried,
        on_grain.state.waste_carried
    );
}

/// An agent standing at the water takes fish out of it.
#[test]
fn an_agent_at_the_water_catches_something() {
    let mut world = a_world_with_a_river();
    world.resources.retain(|r| r.resource_type != ResourceType::Fish);
    world
        .resources
        .push(ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 60));

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.agents[0].state.position = (2, 1, 0);

    let mut simulation = Simulation::new(world, population);

    // Cast until something takes: a river is a sure thing over a few tries,
    // which is the point of it, but no single cast is certain
    let mut caught = 0;
    for _ in 0..40 {
        simulation.population.agents[0].state.position = (2, 1, 0);
        if simulation.execute_action(&Action::Fish, 0).success {
            caught += 1;
        }
    }

    assert!(
        caught > 10,
        "forty casts into a full reach should land a good many; {caught} did"
    );

    let carried = simulation.population.agents[0]
        .inventory
        .get_item("fish")
        .map(|item| item.quantity)
        .unwrap_or(0);

    assert!(carried > 0, "and the fish should be in the pack");

    assert!(
        simulation.population.agents[0].state.waste_carried > 0.0,
        "the guts go into the pack as waste at the waterside"
    );

    let left = simulation
        .world
        .resources
        .iter()
        .find(|r| r.resource_type == ResourceType::Fish)
        .map(|r| r.amount)
        .unwrap_or(0);

    assert!(left < 60, "and they came out of the river");
}

/// Fishing an empty reach teaches an agent that fishing does not pay.
#[test]
fn standing_in_an_empty_river_teaches_something() {
    let mut world = a_world_with_a_river();
    world.resources.retain(|r| r.resource_type != ResourceType::Fish);

    let mut reach = ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 60);
    reach.amount = 0;
    world.resources.push(reach);

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.agents[0].state.position = (2, 1, 0);

    let mut simulation = Simulation::new(world, population);

    for _ in 0..30 {
        simulation.population.agents[0].state.position = (2, 1, 0);
        let result = simulation.execute_action(&Action::Fish, 0);
        simulation.population.agents[0].learn_from(&Action::Fish, result.success);
    }

    assert!(
        !simulation.population.agents[0]
            .lessons
            .worth_trying(Undertaking::Fishing),
        "somebody who has stood in an empty river thirty times stops going"
    );
}

/// A settlement beside a river holds its fields where one inland does not.
///
/// The slow one. It runs two worlds side by side for long enough that the
/// difference is the fishery rather than the weather: the same country, the
/// same people, and in one of them the fish have been taken out of the water.
#[test]
#[ignore = "slow: two worlds to fifteen thousand ticks"]
fn a_river_settlement_keeps_its_ground() {
    fn farmed_fertility(simulation: &Simulation) -> f32 {
        let mut total = 0.0;
        let mut fields = 0;

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
            fields += 1;
        }

        total / fields.max(1) as f32
    }

    let mut with_fish = 0.0;
    let mut without = 0.0;
    const WORLDS: usize = 3;

    for _ in 0..WORLDS {
        let world = World::new(WorldConfig::default());

        // The same country twice, but one of them has no fish in it
        let mut dry = world.clone();
        dry.resources
            .retain(|resource| resource.resource_type != ResourceType::Fish);

        for (world, total) in [(world, &mut with_fish), (dry, &mut without)] {
            let mut population = Population::new();
            for _ in 0..12 {
                population.spawn_agent(AgentConfig::default());
            }
            let mut simulation = Simulation::new(world, population);
            for _ in 0..15_000 {
                simulation.tick();
            }
            *total += farmed_fertility(&simulation);
        }
    }

    let with_fish = with_fish / WORLDS as f32;
    let without = without / WORLDS as f32;

    assert!(
        with_fish > without,
        "a settlement with a river to fish should hold its fields better than \
         the same settlement without one: {with_fish:.3} against {without:.3}"
    );
}
