// src/analytics/tests/midden_tests.rs
//! Tests that a midden smells, is walked away from, and comes up in berries.
//!
//! "Nearly all of the nutrients in the food eaten is returned to the ground as
//! waste. Waste should smell unpleasant and repulse the agents. If the agents
//! are expelling their waste and piling it away from their tents, then over
//! time the waste should break down and seeds from the plants they have eaten
//! should sprout."

use crate::agents::senses::ScentType;
use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::soil::Soil;
use crate::world::{Position, ResourceType, TerrainType, World, WorldConfig};

fn a_world() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    Simulation::new(World::new(WorldConfig::default()), population)
}

/// Foul the ground the agent is standing on, and give it clean neighbours.
fn a_midden_underfoot(simulation: &mut Simulation) -> Position {
    let where_it_is = {
        let here = simulation.population.agents[0].state.position;
        Position::new(here.0, here.1)
    };

    for dy in -4..=4 {
        for dx in -4..=4 {
            let there = Position::new(where_it_is.x + dx, where_it_is.y + dy);
            if let Some(tile) = simulation.world.grid.get_tile_mut(&there) {
                tile.terrain = crate::world::Terrain::new(TerrainType::Plains);
                tile.soil.fouling = 0.0;
            }
        }
    }

    if let Some(tile) = simulation.world.grid.get_tile_mut(&where_it_is) {
        tile.soil.somebody_voided_here(Soil::AS_FOUL_AS_IT_GETS);
    }

    where_it_is
}

// --- what a midden is -------------------------------------------------------

/// Voiding leaves three things: litter, a smell, and seeds.
#[test]
fn what_somebody_passes_is_litter_a_smell_and_seed() {
    let mut soil = Soil::for_terrain(TerrainType::Plains);
    let litter_before = soil.litter();

    soil.somebody_voided_here(1.0);

    assert!(soil.litter() > litter_before, "it is matter on the ground");
    assert!(soil.fouling > 0.0, "and it smells");
    assert!(soil.seeds_dropped > 0.0, "and there is seed in it");
}

/// Most of a berry is digested; the pips are not.
#[test]
fn most_of_what_goes_in_does_not_come_out_able_to_grow() {
    assert!(
        Soil::WHAT_COMES_THROUGH_WHOLE < 0.2,
        "a midden is mostly not seed"
    );
}

/// The smell goes long before the matter does.
#[test]
fn a_midden_stops_smelling_before_it_stops_being_there() {
    let mut soil = Soil::for_terrain(TerrainType::Plains);
    soil.somebody_voided_here(2.0);

    let litter_at_the_start = soil.litter();
    for _ in 0..200 {
        soil.decay(1.0, 12.0);
    }

    assert!(
        !soil.is_foul(),
        "two hundred days of wet weather should take the smell off it: {}",
        soil.fouling
    );
    assert!(
        soil.litter() > litter_at_the_start * 0.1,
        "but the matter is still there, working into the ground"
    );
}

/// Nothing comes up out of a fresh midden.
#[test]
fn nothing_grows_out_of_a_fresh_midden() {
    let mut soil = Soil::for_terrain(TerrainType::Plains);
    soil.somebody_voided_here(Soil::AS_FOUL_AS_IT_GETS);
    soil.nutrients = 0.5;

    assert!(
        !soil.ready_to_sprout(),
        "it has to break down first, which is the whole point"
    );
}

/// And out of a broken-down one, it does.
#[test]
fn what_was_dropped_comes_up_once_the_ground_has_taken_it() {
    let mut soil = Soil::for_terrain(TerrainType::Plains);
    soil.somebody_voided_here(20.0);
    assert!(soil.seeds_dropped >= Soil::ENOUGH_TO_COME_UP);

    for _ in 0..400 {
        soil.decay(1.0, 12.0);
    }

    assert!(
        soil.ready_to_sprout(),
        "seed {:.2}, fouling {:.3}, nutrients {:.3}",
        soil.seeds_dropped,
        soil.fouling,
        soil.nutrients
    );

    let seed = soil.it_came_up();
    assert!(seed > 0.0);
    assert_eq!(soil.seeds_dropped, 0.0, "it only comes up once");
}

// --- in a running world -----------------------------------------------------

/// A midden gives itself away by smell.
#[test]
fn a_midden_can_be_smelt() {
    let mut simulation = a_world();
    a_midden_underfoot(&mut simulation);

    simulation.emit_scents();

    let smelt_something_rotten = simulation.population.agents[0]
        .senses
        .smell
        .detected_scents
        .iter()
        .any(|scent| matches!(scent.scent_type, ScentType::Decay));

    assert!(
        smelt_something_rotten,
        "a man standing on a midden should be able to smell it"
    );
}

/// And nobody lies down in it.
#[test]
fn nobody_sleeps_on_a_midden() {
    let mut simulation = a_world();
    let midden = a_midden_underfoot(&mut simulation);
    let standing_on_it = (midden.x, midden.y, 0);

    {
        let agent = &mut simulation.population.agents[0];
        agent.fatigue.is_sleeping = false;
    }

    let action = {
        let agent = &simulation.population.agents[0];
        simulation.what_this_drive_offers(DriveType::Rest, agent, standing_on_it)
    };

    match action {
        Some(Action::Move { target }) => {
            let there = Position::new(target.0, target.1);
            assert!(
                !simulation
                    .world
                    .grid
                    .get_tile(&there)
                    .map(|tile| tile.soil.is_foul())
                    .unwrap_or(true),
                "he should move to ground that does not stink"
            );
        }
        other => panic!("a man on a midden should step off it, not {other:?}"),
    }
}

/// On clean ground he simply lies down.
#[test]
fn on_clean_ground_a_tired_man_just_sleeps() {
    let mut simulation = a_world();
    let here = simulation.population.agents[0].state.position;

    if let Some(tile) = simulation.world.grid.get_tile_mut(&Position::new(here.0, here.1)) {
        tile.soil.fouling = 0.0;
    }
    simulation.population.agents[0].fatigue.is_sleeping = false;

    let action = {
        let agent = &simulation.population.agents[0];
        simulation.what_this_drive_offers(DriveType::Rest, agent, here)
    };

    assert!(
        matches!(action, Some(Action::Sleep { .. })),
        "nothing to walk away from, so he sleeps: {action:?}"
    );
}

/// The whole thing: a fouled tile, left alone, comes up in something edible.
#[test]
fn a_midden_left_alone_comes_up_in_food() {
    let mut simulation = a_world();
    let midden = a_midden_underfoot(&mut simulation);

    // Enough seed for something to come of it, and clear the ground so that
    // what appears can only have come from the midden.
    simulation
        .world
        .resources
        .retain(|resource| resource.position != midden);

    if let Some(tile) = simulation.world.grid.get_tile_mut(&midden) {
        tile.soil.somebody_voided_here(20.0);
        // And let it break down, which is what the seasons would do.
        for _ in 0..400 {
            tile.soil.decay(1.0, 12.0);
        }
    }

    simulation.what_was_dropped_comes_up();

    let came_up = simulation
        .world
        .resources
        .iter()
        .find(|resource| resource.position == midden);

    let came_up = came_up.expect("something should have come up on the midden");
    assert_eq!(came_up.resource_type, ResourceType::Food);
    assert!(came_up.amount > 0, "and it should be worth picking");
}

/// It does not come up on top of something already growing there.
#[test]
fn nothing_comes_up_where_something_already_grows() {
    let mut simulation = a_world();
    let midden = a_midden_underfoot(&mut simulation);

    simulation
        .world
        .resources
        .retain(|resource| resource.position != midden);
    simulation.world.resources.push(crate::world::ResourceNode::new(
        ResourceType::Wood,
        midden,
        50,
    ));

    if let Some(tile) = simulation.world.grid.get_tile_mut(&midden) {
        tile.soil.somebody_voided_here(20.0);
        for _ in 0..400 {
            tile.soil.decay(1.0, 12.0);
        }
    }

    simulation.what_was_dropped_comes_up();

    let here: Vec<_> = simulation
        .world
        .resources
        .iter()
        .filter(|resource| resource.position == midden)
        .collect();

    assert_eq!(here.len(), 1, "the tree is still there and nothing else");
    assert_eq!(here[0].resource_type, ResourceType::Wood);
}

/// A settlement that lives somewhere fouls the ground it lives on.
#[test]
fn a_settlement_fouls_the_ground_it_stands_on() {
    let mut population = Population::new();
    for _ in 0..8 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);

    for _ in 0..400 {
        simulation.tick();
    }

    let fouled = simulation
        .world
        .grid
        .tiles
        .iter()
        .flat_map(|row| row.iter())
        .filter(|tile| tile.soil.fouling > 0.0)
        .count();

    assert!(
        fouled > 0,
        "eight people living for a month should have left something behind them"
    );
}
